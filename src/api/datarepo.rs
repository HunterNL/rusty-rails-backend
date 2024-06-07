use std::{
    collections::{HashMap, HashSet},
    fs::File,
    hash::Hash,
    iter,
};

use chrono::{NaiveDate, NaiveTime};
mod links;
pub mod stations;
use crate::{
    api::datarepo::{links::extract_links, stations::extract_stations},
    dayoffset::DayOffset,
    iff::{self, Iff, Leg, LegKind, Record, Ride},
};

use self::{links::Link, stations::Station};

/// A master container for all data, this is the struct eventually passed to the server
pub struct DataRepo {
    links: Vec<Link>,
    stations: Vec<stations::Station>,
    // rides: Vec<Record>,
    // link_map: HashMap<LinkCode, Link>,
    iff: Iff,
    rides: Vec<iff::Ride>,
}

/// Key to identify links, looking up links with the waypoint identifiers the wrong way around should return a corrected Link
#[derive(Eq, Hash, PartialEq)]
pub struct LinkCode(String, String);

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
        LegKind::Moving(from, to, waypoints) => Some({
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
        LegKind::Moving(_from, _to, _waypoints) => leg_codes(&leg.kind)
            .iter()
            .all(|leg_code| leg_code.iter().all(|code| links.contains_undirected(code))),
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

        let mut iff = Iff::new_from_archive(&iff_file)
            .map_err(|e| println!("{e}"))
            .expect("valid parse");

        let route_file =
            File::open(cache_dir.join("remote").join("route.json")).expect("To find route file");
        let stations_file = File::open(cache_dir.join("remote").join("stations.json"))
            .expect("To find stations file");

        let links: Vec<Link> = extract_links(&route_file);
        let link_map: HashMap<LinkCode, Link> = links
            .iter()
            .map(|link| (link.link_code(), link.clone()))
            .collect();
        let stations = extract_stations(&stations_file);

        let station_codes: HashSet<String> = stations.iter().map(|s| s.code.clone()).collect();

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

        // TODO Drop this check and deal with skipping waypoints throughout the app, or deal with translating stations from the iff into coordinates
        // This filters out timetable entries that contain stops that we don't have data on, mostly (entirely?) international trains
        println!("Pre data filter ride #: {}", iff.timetable().rides.len());
        iff.timetable_mut()
            .rides
            .retain(|ride| has_complete_data(ride, &station_codes, &link_map));

        let rides: Vec<iff::Ride> = iff
            .timetable()
            .rides
            .iter()
            .flat_map(|record| record.split_on_ride_id())
            .collect();

        // let links_map=  HashMap::from_iter(links.iter().map(|link| (make_link_code(link., b))))

        // timetable.rides.retain(|ride| {
        //     ride.timetable.windows(2).all(|slice| {
        //         let [left,right] = slice else {panic!("unexpected match failure")};

        //     })
        // })

        println!("Post data filter ride #: {}", iff.timetable().rides.len());

        // println!("{:?}", stations);

        Self {
            rides,
            links,
            stations,
            // link_map,
            iff,
        }
    }

    pub fn rides(&self) -> &[Ride] {
        &self.rides
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
