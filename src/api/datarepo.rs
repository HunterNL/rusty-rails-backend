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
    iff::{self, Company, Iff, Leg, LegKind, Record, Ride},
};

use self::{links::Link, stations::Station};

/// A master container for all data, this is the struct eventually passed to the server
pub struct DataRepo {
    links: Vec<Link>,
    stations: Vec<stations::Station>,
    iff: Iff,
    rides: Vec<iff::Ride>,
}

/// Key to identify links, looking up links with the waypoint identifiers the wrong way around should return a corrected Link
#[derive(Eq, Hash, PartialEq)]
pub struct LinkCode(String, String);

#[derive(Hash, PartialEq, Eq)]
pub enum MissingLinkReport {
    NoRoute(String, String),
    NoStation(String),
}

impl Display for MissingLinkReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MissingLinkReport::NoRoute(from, to) => {
                f.write_fmt(format_args!("no route from {} to {}", from, to))
            }
            MissingLinkReport::NoStation(code) => {
                f.write_fmt(format_args!("no station code {code}"))
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
        self.contains_key(code) || self.contains_key(&LinkCode(code.1.clone(), code.0.clone()))
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
                .collect::<Vec<&String>>()
                .windows(2)
                .map(|slice| LinkCode(slice[0].to_string(), slice[1].to_string()))
                .collect()
        }),
    }
}

fn leg_has_complete_data(
    leg: &Leg,
    station_codes: &HashSet<String>,
    links: &HashMap<LinkCode, Link>,
) -> bool {
    match &leg.kind {
        LegKind::Stationary(code, _) => station_codes.contains(code),
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
    links: &HashMap<LinkCode, Link>,
) -> Vec<MissingLinkReport> {
    record
        .generate_legs()
        .iter()
        .flat_map(|leg| report_missing_leg(leg, station_codes, links))
        .collect()
}

fn report_missing_leg(
    leg: &Leg,
    station_codes: &HashSet<String>,
    links: &HashMap<LinkCode, Link>,
) -> Option<MissingLinkReport> {
    match &leg.kind {
        LegKind::Stationary(code, _) => (!station_codes.contains(code.as_str()))
            .then(|| MissingLinkReport::NoStation(code.clone())),
        LegKind::Moving {
            from,
            to,
            waypoints: _,
        } => (!links.contains_undirected(&LinkCode(from.clone(), to.clone())))
            .then(|| MissingLinkReport::NoRoute(from.clone(), to.clone())),
    }
}

fn has_complete_data(
    record: &Record,
    station_codes: &HashSet<String>,
    links: &HashMap<LinkCode, Link>,
) -> bool {
    record
        .generate_legs()
        .iter()
        .all(|leg| leg_has_complete_data(leg, station_codes, links))
}

impl DataRepo {
    pub fn new(cache_dir: &std::path::Path) -> Self {
        let iff_file = File::open(cache_dir.join("remote").join("ns-latest.zip"))
            .expect("To find timetable file");

        let iff = Iff::new_from_archive(&iff_file)
            .map_err(|e| println!("{e}"))
            .expect("valid parse");

        let route_file =
            File::open(cache_dir.join("remote").join("route.json")).expect("To find route file");
        let stations_file = File::open(cache_dir.join("remote").join("stations.json"))
            .expect("To find stations file");

        let links: Vec<Link> = extract_links(&route_file);

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
            .flat_map(|record| record.split_on_ride_id())
            .collect();

        Self {
            rides,
            links,
            stations,
            // link_map,
            iff,
        }
    }

    pub fn report_unkown_legs(&self) {
        let link_map: HashMap<LinkCode, Link> = self
            .links
            .iter()
            .map(|link| (link.link_code(), link.clone()))
            .collect();

        let station_codes: HashSet<String> = self.stations.iter().map(|s| s.code.clone()).collect();

        let reports: Vec<_> = self
            .iff
            .timetable()
            .rides
            .iter()
            .filter(|r| !has_complete_data(r, &station_codes, &link_map))
            .flat_map(|r| report_missing(r, &station_codes, &link_map))
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
            .for_each(|e| println!("{} ({})", e.0, e.1))
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

        self.iff
            .timetable_mut()
            .rides
            .retain(|ride| has_complete_data(ride, &station_codes, &link_map));

        println!(
            "Post data filter ride #: {}",
            self.iff.timetable().rides.len()
        );

        self.rides = self
            .iff
            .timetable()
            .rides
            .iter()
            .flat_map(|record| record.split_on_ride_id())
            .collect();
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

    pub fn is_ride_valid(&self, footnote: u64, day: NaiveDate) -> bool {
        self.iff.validity().is_valid_on_day(footnote, day).unwrap()
    }
}
