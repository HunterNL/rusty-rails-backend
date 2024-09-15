use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    fs::File,
    hash::Hash,
    iter,
};

use chrono::{NaiveDate, NaiveTime};
mod links;
mod stations;
use crate::{
    api::datarepo::{links::extract_links, stations::extract_stations},
    dayoffset::DayOffset,
    fetch::{ROUTE_FILEPATH, STATION_FILEPATH, TIMETABLE_PATH},
    iff::{self, Company, Iff, Leg, LegKind, LocationCache, LocationCodeHandle, Record, Ride},
};

use self::{links::Link, stations::Station};

// use super::ApiSerializationContext;

/// A master container for all data, this is the struct eventually passed to the server
pub struct DataRepo {
    links: Vec<Link>,
    stations: Vec<stations::Station>,
    iff: Iff,
    rides: Vec<iff::Ride>,
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

impl<'a, 'b> Display for MissingLinkReportDisplay<'a, 'b> {
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

    return match candidate_matches.len() {
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
    };
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
            "Timetable end date: {}",
            iff.timetable().header.last_valid_date
        );
        println!("Day count: {}", duration.num_days());

        let rides: Vec<iff::Ride> = iff
            .timetable()
            .rides
            .iter()
            .flat_map(|r| r.split_on_ride_id())
            .collect();

        let version = iff.header().version;

        Self {
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

    pub fn filter_unknown_legs(&mut self) {
        // TODO Drop this check and deal with skipping waypoints throughout the app, or deal with translating stations from the iff into coordinates
        // This filters out timetable entries that contain stops that we don't have data on, mostly (entirely?) international trains
        println!(
            "Pre data filter ride #: {}",
            self.iff.timetable().rides.len()
        );

        let link_map: HashMap<LinkCode, Link> = self
            .links
            .iter()
            .map(|link| (link.link_code(), link.clone()))
            .collect();

        let station_codes: HashSet<String> = self.stations.iter().map(|s| s.code.clone()).collect();
        let location_cache = &self.iff.locations.clone(); // Clone is safe since it's only being

        self.iff
            .rides_mut()
            .retain(|ride| has_complete_data(ride, &station_codes, location_cache, &link_map));

        println!(
            "Post data filter ride #: {}",
            self.iff.timetable().rides.len()
        );

        self.rides = self
            .iff
            .timetable()
            .rides
            .iter()
            .flat_map(|r| r.split_on_ride_id())
            .collect()
    }

    pub fn rides(&self) -> &[Ride] {
        &self.rides
    }

    pub fn companies(&self) -> &[Company] {
        self.iff.companies()
    }

    pub fn rides_active_at_time(&self, time: &NaiveTime, date: &NaiveDate) -> Vec<&Ride> {
        let time = DayOffset::from_naivetime(time);

        self.rides()
            // .timetable()
            // .rides
            .iter()
            .filter(|r| r.start_time() < time && r.end_time() > time)
            .filter(|r| {
                self.iff
                    .validity()
                    .is_valid_on_day(r.day_validity, *date)
                    .unwrap()
            })
            // .cloned()
            .collect()
    }

    pub fn rides_active_in_timespan(
        &self,
        time_start: &NaiveTime,
        time_end: &NaiveTime,
        date: &NaiveDate,
    ) -> Vec<&Ride> {
        let offset_start = DayOffset::from_naivetime(time_start);
        let offset_end = DayOffset::from_naivetime(time_end);

        self.rides()
            // .timetable()
            // .rides
            .iter()
            .filter(|r| r.start_time() <= offset_end && r.end_time() > offset_start)
            .filter(|r| {
                self.iff
                    .validity()
                    .is_valid_on_day(r.day_validity, *date)
                    .unwrap()
            })
            // .cloned()
            .collect()
    }

    pub fn rides_active_on_date(&self, date: &NaiveDate) -> Vec<&Ride> {
        self.rides()
            .iter()
            .filter(|r| {
                self.iff
                    .validity()
                    .is_valid_on_day(r.day_validity, *date)
                    .unwrap()
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

    pub fn is_ride_valid(&self, footnote: u64, day: NaiveDate) -> bool {
        self.iff.validity().is_valid_on_day(footnote, day).unwrap()
    }

    pub fn location_cache(&self) -> &LocationCache {
        &self.iff.locations
    }
}
