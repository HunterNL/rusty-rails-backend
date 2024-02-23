use std::{
    collections::{HashMap, HashSet},
    fs::File,
    hash::Hash,
    iter,
};

use chrono::{NaiveDate, NaiveTime};
mod links;
mod stations;
use crate::{
    api::datarepo::{links::extract_links, stations::extract_stations},
    iff::{
        self,
        dayoffset::DayOffset,
        parsing::{Leg, LegKind, Record},
    },
};

use self::{links::Link, stations::Station};

/// A master container for all data, this is the struct eventually passed to the server
pub struct DataRepo {
    links: Vec<Link>,
    stations: Vec<stations::Station>,
    rides: Vec<Record>,
    link_map: HashMap<LinkCode, Link>,
    validity: iff::parsing::RideValidity,
}

/// Key to identify links, looking up links with the waypoint identifiers the wrong way around should return a corrected Link
#[derive(Eq, Hash, PartialEq)]
pub struct LinkCode(String, String);

trait LinkMap {
    fn get_undirected(&self, code: &LinkCode) -> Option<(&Link, bool)>;
    fn contains_undirected(&self, code: &LinkCode) -> bool;
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
/// Takes a Leg reference, if it is a moving leg: returns all the LinkCodes required to traverse this leg
fn leg_codes(leg: &LegKind) -> Option<Vec<LinkCode>> {
    match leg {
        LegKind::Stationary(_) => None,
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
        iff::parsing::LegKind::Stationary(code) => station_codes.contains(code),
        iff::parsing::LegKind::Moving(_from, _to, _waypoints) => leg_codes(&leg.kind)
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
        let route_file =
            File::open(cache_dir.join("remote").join("route.json")).expect("To find route file");
        let stations_file = File::open(cache_dir.join("remote").join("stations.json"))
            .expect("To find stations file");

        let timetable = iff::parsing::Iff::timetable(&iff_file);
        let validity = iff::parsing::Iff::validity(&iff_file);

        if timetable.is_err() {
            eprint!("{}", timetable.err().unwrap());
            panic!("Error parsing timetable");
        }

        if validity.is_err() {
            eprint!("{}", validity.err().unwrap());
            panic!("Error parsing validity");
        }

        let mut timetable = timetable.unwrap();
        let validity = validity.unwrap();

        let links: Vec<Link> = extract_links(&route_file);
        let link_map: HashMap<LinkCode, Link> = links
            .iter()
            .map(|link| (link.link_code(), link.clone()))
            .collect();
        let stations = extract_stations(&stations_file);

        let station_codes: HashSet<String> = stations.iter().map(|s| s.code.clone()).collect();

        let duration = timetable
            .header
            .last_valid_date
            .signed_duration_since(timetable.header.first_valid_date);

        println!(
            "Timetable start date: {}",
            timetable.header.first_valid_date
        );
        println!("Timetable end date: {}", timetable.header.last_valid_date);
        println!("Day count: {}", duration.num_days());

        // TODO Drop this check and deal with skipping waypoints throughout the app, or deal with translating stations from the iff into coordinates
        //
        // This filters out timetable entries that contain stops that we don't have data on, mostly (entirely?) international trains

        println!("Pre data filter ride #: {}", timetable.rides.len());
        timetable
            .rides
            .retain(|ride| has_complete_data(ride, &station_codes, &link_map));

        // let links_map=  HashMap::from_iter(links.iter().map(|link| (make_link_code(link., b))))

        // timetable.rides.retain(|ride| {
        //     ride.timetable.windows(2).all(|slice| {
        //         let [left,right] = slice else {panic!("unexpected match failure")};

        //     })
        // })

        println!("Post data filter ride #: {}", timetable.rides.len());

        // println!("{:?}", stations);

        Self {
            links,
            link_map,
            stations,
            rides: timetable.rides,
            validity,
        }
    }

    pub fn rides_active_at_time(&self, time: &NaiveTime, date: &NaiveDate) -> Vec<Record> {
        let time = DayOffset::from_naivetime(time);

        println!("{time:?}");
        println!("{}", self.rides.len());
        self.rides
            .iter()
            .filter(|r| r.start_time() < time && r.end_time() > time)
            .filter(|r| {
                self.validity
                    .is_valid_on_day(r.day_validity_footnote.footnote, *date)
                    .unwrap()
            })
            .cloned()
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
}
