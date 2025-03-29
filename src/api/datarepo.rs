use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    fs::File,
    hash::Hash,
    iter,
};

use chrono::{Days, NaiveDate, NaiveDateTime, NaiveTime};
mod links;
mod stations;
use crate::ride_recurrance::RideRecurrence;
use crate::{
    api::datarepo::{links::extract_links, stations::extract_stations},
    dayoffset::DayOffset,
    fetch::{ROUTE_FILEPATH, STATION_FILEPATH, TIMETABLE_PATH},
    iff::{Company, Iff, Leg, LegKind, LocationCache, LocationCodeHandle, Record},
    ride::Ride,
};

use self::{links::Link, stations::Station};

// use super::ApiSerializationContext;

/// A master container for all data, this is the struct eventually passed to the server
pub struct DataRepo {
    links: Vec<Link>,
    stations: Vec<stations::Station>,
    iff: Iff,
    rides: Vec<RideRecurrence>,
    rides_by_day: HashMap<NaiveDate, Vec<RideRecurrence>>,
    day_stats: HashMap<NaiveDate, Daymeta>,
    version: u64,
}

/// Key to identify links, looking up links with the waypoint identifiers the wrong way around should return a corrected Link
#[derive(Eq, Hash, PartialEq, Debug)]
pub struct LinkCode(LocationCodeHandle, LocationCodeHandle);

#[derive(Hash, PartialEq, Eq)]
pub enum MissingLinkReport {
    NoRoute(LocationCodeHandle, LocationCodeHandle),
    NoStation(LocationCodeHandle),
}

struct MissingLinkReportDisplay<'a, 'b> {
    inner: &'a MissingLinkReport,
    cache: &'b LocationCache,
}

impl MissingLinkReport {
    fn display<'a, 'b>(&'a self, cache: &'b LocationCache) -> MissingLinkReportDisplay<'a, 'b> {
        MissingLinkReportDisplay { inner: self, cache }
    }
}

#[derive(Debug, Clone)]
struct Daymeta {
    first_ride_start: DayOffset,
    last_ride_end: DayOffset,
    ride_count: usize,
}

impl Display for MissingLinkReportDisplay<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner {
            MissingLinkReport::NoRoute(from, to) => {
                let from = self.cache.get_str(from).unwrap();
                let to = self.cache.get_str(to).unwrap();
                f.write_fmt(format_args!("no route from {} to {}", from, to))
            }
            MissingLinkReport::NoStation(code) => {
                let name = self.cache.get_str(code).unwrap();
                f.write_fmt(format_args!("no station code {name}"))
            }
        }
    }
}

trait LinkMap {
    #[allow(dead_code)]
    fn get_undirected(&self, code: &LinkCode) -> Option<(&Link, bool)>;
    fn contains_undirected(&self, code: &LinkCode) -> bool;
    #[allow(dead_code)]
    fn contains_directed(&self, code: &LinkCode) -> bool;
}

impl LinkMap for HashMap<LinkCode, Link> {
    fn get_undirected(&self, _code: &LinkCode) -> Option<(&Link, bool)> {
        // let normal = self.get(&make_link_code(a, b));
        // if normal.is_some() {
        //     return Some((normal.unwrap(), false));
        // }

        // let inverted = self.get(&make_link_code(b, a));
        // if inverted.is_some() {
        //     return Some((inverted.unwrap(), true));
        // }

        // None

        todo!()
    }

    fn contains_undirected(&self, code: &LinkCode) -> bool {
        self.contains_key(code) || self.contains_key(&LinkCode(code.1, code.0))
    }

    fn contains_directed(&self, code: &LinkCode) -> bool {
        self.contains_key(code)
    }
}
/// Takes a Leg reference, if it is a moving leg: returns all the `LinkCodes` required to traverse this leg
fn leg_codes(leg: &LegKind) -> Option<Vec<LinkCode>> {
    match leg {
        LegKind::Stationary(_, _) => None,
        LegKind::Moving {
            from,
            to,
            waypoints,
        } => Some({
            iter::once(from)
                .chain(waypoints.iter())
                .chain(iter::once(to))
                .collect::<Vec<_>>()
                .windows(2)
                .map(|slice| LinkCode(*slice[0], *slice[1]))
                .collect()
        }),
    }
}

fn leg_has_complete_data(
    leg: &Leg,
    station_codes: &HashSet<String>,
    location_cache: &LocationCache,
    links: &HashMap<LinkCode, Link>,
) -> bool {
    match &leg.kind {
        LegKind::Stationary(location, _) => {
            let code = location_cache.get_str(location).unwrap();
            station_codes.contains(code)
        }
        LegKind::Moving {
            from: _from,
            to: _to,
            waypoints: _waypoints,
        } => leg_codes(&leg.kind)
            .iter()
            .all(|leg_code| leg_code.iter().all(|code| links.contains_undirected(code))),
    }
}

fn report_missing(
    record: &Record,
    station_codes: &HashSet<String>,
    location_cache: &LocationCache,
    links: &HashMap<LinkCode, Link>,
) -> Vec<MissingLinkReport> {
    record
        .generate_legs()
        .iter()
        .flat_map(|leg| report_missing_leg(leg, station_codes, location_cache, links))
        .collect()
}

fn report_missing_leg(
    leg: &Leg,
    station_codes: &HashSet<String>,
    location_cache: &LocationCache,
    links: &HashMap<LinkCode, Link>,
) -> Option<MissingLinkReport> {
    match &leg.kind {
        LegKind::Stationary(location, _) => {
            let code = location_cache.get_str(location).unwrap();
            (!station_codes.contains(code)).then_some(MissingLinkReport::NoStation(*location))
        }
        LegKind::Moving {
            from,
            to,
            waypoints: _,
        } => (!links.contains_undirected(&LinkCode(*from, *to)))
            .then_some(MissingLinkReport::NoRoute(*from, *to)),
    }
}

fn has_complete_data(
    record: &Record,
    station_codes: &HashSet<String>,
    location_cache: &LocationCache,
    links: &HashMap<LinkCode, Link>,
) -> bool {
    record
        .generate_legs()
        .iter()
        .all(|leg| leg_has_complete_data(leg, station_codes, location_cache, links))
}

pub fn select_station_by_name<'a>(stations: &'a [Station], needle: &str) -> Option<&'a Station> {
    let needle = needle.to_lowercase();

    let exact_match = stations.iter().find(|s| s.name.to_lowercase() == needle);

    if exact_match.is_some() {
        return exact_match;
    }

    let candidate_matches: Vec<_> = stations
        .iter()
        .map(|s| (s, s.name.to_lowercase()))
        .filter(|(_, name)| name.contains(needle.as_str()))
        .collect();

    match candidate_matches.len() {
        0 => None,
        1 => candidate_matches
            .first()
            .map(|(station, _): &(&Station, String)| *station),
        _ => {
            println!("Got plenty of matches, figure out some heuristics");

            for station in candidate_matches.iter() {
                println!("{}", station.0.name)
            }

            candidate_matches.first().map(|a| a.0)
        }
    }
}

impl DataRepo {
    pub fn new(cache_dir: &std::path::Path) -> Self {
        let iff_file = File::open(cache_dir.join(TIMETABLE_PATH)).expect("To find timetable file");

        let mut iff = Iff::new_from_archive(&iff_file)
            .map_err(|e| println!("{e}"))
            .expect("valid parse");

        let route_file = File::open(cache_dir.join(ROUTE_FILEPATH)).expect("To find route file");
        let stations_file =
            File::open(cache_dir.join(STATION_FILEPATH)).expect("To find stations file");

        let links: Vec<Link> = extract_links(&route_file, &mut iff.locations);

        let stations = extract_stations(&stations_file);

        let duration = iff
            .timetable()
            .header
            .last_valid_date
            .signed_duration_since(iff.timetable().header.first_valid_date);

        println!(
            "Timetable start date: {}",
            iff.timetable().header.first_valid_date
        );
        println!(
            "Timetable end date:   {}",
            iff.timetable().header.last_valid_date
        );
        println!("Day count: {}", duration.num_days());
        println!("Version: {}", iff.header().version);

        let rides = Self::create_valid_rides(iff.rides(), &links, &stations, &iff.locations);
        let version = iff.header().version;
        let mut rides_by_day = HashMap::new();

        println!("DR 1");

        // BAD
        iff.timetable()
            .header
            .first_valid_date
            .iter_days()
            .take_while(|d| d <= &iff.timetable().header.last_valid_date)
            .for_each(|date| {
                let rides_on_day: Vec<_> = rides
                    .iter()
                    .enumerate()
                    .filter(|(_, ride)| {
                        iff.validity()
                            .is_valid_on_day(ride.day_validity, &date)
                            .unwrap()
                    })
                    .map(|a| a.1.clone())
                    .collect();
                rides_by_day.insert(date, rides_on_day);

                // rides.iter().enumerate().filter(|(index, ride)| {
                //     iff.validity()
                //         .is_valid_on_day(ride.day_validity, &date)
                //         .unwrap()
                // })
            });

        println!("DR 2");

        // c                rides.iter().enumerate().filter_map(|(index,ride)|{

        // });
        //
        let daily_stats: HashMap<_, _> = rides_by_day
            .iter()
            .map(|(day, indexes)| {
                let min = indexes
                    .iter()
                    // .map(|i| rides.get(*i).unwrap())
                    .min_by_key(|r| r.departure_time())
                    .map(|r| r.departure_time())
                    .unwrap();

                let max = indexes
                    .iter()
                    // .map(|i| rides.get(*i).expect("recurrence to refer to valid ride id"))
                    .max_by_key(|r| r.arrival_time())
                    .map(|r| r.departure_time())
                    .unwrap();

                let count = indexes.len();

                let dm: Daymeta = Daymeta {
                    first_ride_start: min,
                    last_ride_end: max,
                    ride_count: count,
                };

                (*day, dm)
            })
            .collect();

        println!("DR 3");

        let mut temp: Vec<(&NaiveDate, &Daymeta)> = daily_stats.iter().collect();
        temp.sort_unstable_by_key(|a| a.0);
        temp.iter().for_each(|(date, stats)| {
            println!(
                "{} {} {} {}",
                date,
                stats.first_ride_start.display_unwrapped(),
                stats.last_ride_end.display_unwrapped(),
                stats.ride_count,
            )
        });

        println!("Finished creating datarepo");

        Self {
            day_stats: daily_stats,

            rides_by_day,
            rides,
            links,
            stations,
            // link_map,
            iff,
            version,
        }
    }

    pub fn report_unkown_legs(&self) {
        let link_map: HashMap<LinkCode, Link> = self
            .links
            .iter()
            .map(|link| (link.link_code(), link.clone()))
            .collect();

        let station_codes: HashSet<String> = self.stations.iter().map(|s| s.code.clone()).collect();
        let location_cache = &self.iff.locations;

        let reports: Vec<_> = self
            .iff
            .timetable()
            .rides
            .iter()
            .filter(|r| !has_complete_data(r, &station_codes, location_cache, &link_map))
            .flat_map(|r| report_missing(r, &station_codes, location_cache, &link_map))
            .collect();

        let mut map = HashMap::new();

        reports
            .into_iter()
            .for_each(|r| *map.entry(r).or_insert(0) += 1);

        let mut entries: Vec<_> = map.into_iter().collect();
        entries.sort_by_key(|r| r.1);
        entries
            .iter()
            .rev()
            .for_each(|e| println!("{} ({})", e.0.display(location_cache), e.1))
    }

    pub fn rides_active_on_day(&self, date: &NaiveDate) -> Vec<Ride<'_>> {
        self.rides_by_day
            .get(date)
            .iter()
            .flat_map(|a| a.iter())
            .map(|r| Ride {
                recurrence: r,
                date: *date,
            })
            .collect()
    }

    pub fn create_valid_rides(
        record: &[Record],
        links: &[Link],
        stations: &[Station],
        location_cache: &LocationCache,
    ) -> Vec<RideRecurrence> {
        // TODO Drop this check and deal with skipping waypoints throughout the app, or deal with translating stations from the iff into coordinates
        // This filters out timetable entries that contain stops that we don't have data on, mostly (entirely?) international trains
        // println!(
        //     "Pre data filter ride #:  {}",
        //     self.iff.timetable().rides.len()
        // );

        let link_map: HashMap<LinkCode, Link> = links
            .iter()
            .map(|link| (link.link_code(), link.clone()))
            .collect();

        let station_codes: HashSet<String> = stations.iter().map(|s| s.code.clone()).collect();
        // let location_cache = self.iff.locations.clone(); // Clone is safe since it's only being

        let records: Vec<&Record> = record
            .iter()
            .filter(|ride| has_complete_data(ride, &station_codes, location_cache, &link_map))
            .collect();

        // println!(
        //     "Post data filter ride #: {}",
        //     self.iff.timetable().rides.len()
        // );

        records.iter().flat_map(|r| r.split_on_ride_id()).collect()
    }

    pub fn rides(&self) -> &[RideRecurrence] {
        &self.rides
    }

    pub fn companies(&self) -> &[Company] {
        self.iff.companies()
    }

    pub fn is_ride_active_on_day(&self, date: &NaiveDate, ride: &RideRecurrence) -> bool {
        self.iff
            .validity()
            .is_valid_on_day(ride.day_validity, date)
            .expect("valid footnote")
    }

    pub fn rides_active_at_time(&self, time: &NaiveTime, date: &NaiveDate) -> Vec<Ride> {
        let time = DayOffset::from_naivetime(time);

        self.rides()
            .iter()
            .filter(|ride| ride.start_time() < time && ride.end_time() > time)
            .filter(|ride| self.is_ride_active_on_day(date, ride))
            .map(|r| Ride {
                date: *date,
                recurrence: r,
            })
            .collect()
    }

    pub fn rides_active_in_timespan(
        &self,
        time_start: &NaiveDateTime,
        time_end: &NaiveDateTime,
    ) -> Vec<Ride> {
        // let time_current = time.stc
        let date_current = time_start.date();
        let date_yesterday = date_current.checked_sub_days(Days::new(1)).unwrap();
        // let date_yesterday = date_current
        //     .checked_sub_days(Days::new(1))
        //     .expect("could find yesterday");

        // let rides_yesterday = iter::once(date_yesterday).map(|date| self.iff.validity().day_index_from_date(&date)).flat_map(|index| {
        // self
        // })
        //
        //
        println!("{}", date_current);
        let mut rides_today = self.rides_active_on_day(&date_current);
        rides_today.retain(|a| {
            a.recurrence
                .is_active_in_timespan(time_start.time().into(), time_end.time().into())
        });

        let mut time_start_yesterday: DayOffset = time_start.time().into();
        let mut time_end_yesterday: DayOffset = time_end.time().into();

        time_start_yesterday = time_start_yesterday.offset_by_days(1).unwrap();
        time_end_yesterday = time_end_yesterday.offset_by_days(1).unwrap();

        let mut rides_yesterday = self.rides_active_on_day(&date_yesterday);
        rides_yesterday.retain(|ride| {
            ride.recurrence
                .is_active_in_timespan(time_start_yesterday, time_end_yesterday)
        });

        // rides_yesterday.clear();

        rides_today.into_iter().chain(rides_yesterday).collect()

        // [(date_current,0),(date_yesterday,60*24)].iter().map(|(date,offset)| self.iff.validity().day_index_from_date(date)).flat_map(|day_index| {
        //      self.rides_by_day.get(day_index)
        //  }).flat_map(|ride_indx|{
        //          ride_indx.iter().flat_map(|r|self.rides.get(*r))
        //      }).map(|a|)
    }

    pub fn rides_active_on_date(&self, date: &NaiveDate) -> Vec<Ride> {
        self.rides()
            .iter()
            .filter(|r| {
                self.iff
                    .validity()
                    .is_valid_on_day(r.day_validity, date)
                    .unwrap()
            })
            .map(|r| Ride {
                date: *date,
                recurrence: r,
            })
            .collect()
    }

    pub fn links(&self) -> &[Link] {
        &self.links //[0..1]
                    // .iter()
                    // .filter(|link| link.link_code() == LinkCode("ac".to_owned(), "bkl".to_owned()))
                    // .collect::<Vec<Link>>()
                    // .as_slice()
    }

    pub fn stations(&self) -> &[Station] {
        &self.stations
    }

    pub fn station_by_code(&self, code: impl AsRef<str>) -> Option<&Station> {
        let code = code.as_ref();
        self.stations.iter().find(|station| station.code == code)
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn is_ride_valid(&self, footnote: u64, day: &NaiveDate) -> bool {
        self.iff.validity().is_valid_on_day(footnote, day).unwrap()
    }

    pub fn location_cache(&self) -> &LocationCache {
        &self.iff.locations
    }

    // pub fn live_ride(
    //     &self,
    //     localtime: &NaiveDateTime,
    //     reccurence: &RideRecurrence,
    // ) -> Option<Ride> {
    // }
}
