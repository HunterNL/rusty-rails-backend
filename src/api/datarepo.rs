use std::{f32::consts::PI, fs::File, io::BufReader};

use serde::{Deserialize, Serialize};
mod links;
mod stations;
use crate::{api::datarepo::stations::extract_stations, iff};

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
pub struct Coords2D {
    lon: f32,
    lat: f32,
}

impl Coords2D {
    pub fn new(lon: f32, lat: f32) -> Self {
        Self { lon, lat }
    }
}

#[derive(Debug)]
pub struct Link {
    // id: u32,
    from: String,
    to: String,
    path: Path,
}

// const EARTH_RADIUS: f32 = 12742f32;

// https://stackoverflow.com/a/21623206
// fn greatCircleDistance(lat1, lon1, lat2, lon2 float64) float64 {
fn great_circle_distance(coords1: &Coords2D, coords2: &Coords2D) -> f32 {
    let radius: f32 = 6371f32; // km
    let p: f32 = std::f32::consts::PI / 180f32;

    let a = 0.5f32 - ((coords2.lat - coords1.lat) * p).cos() / 2f32
        + (coords1.lat * p).cos()
            * (coords2.lat * p).cos()
            * (1f32 - ((coords2.lon - coords1.lon) * p).cos())
            / 2f32;

    2f32 * radius * a.sqrt().asin()

    // return 2 * r * Math.asin(Math.sqrt(a));
}

// fn great_circle_distance(coord1: &Coords2D, coord2: &Coords2D) -> f32 {
//     let p = std::f32::consts::PI / 180f32;
//     // var p = 0.017453292519943295 // Math.PI / 180
//     // var c = Math.cos
//     let a = 0.5f32 - ((coord2.lat - coord1.lat).cos() * p) / 2f32
//         + (coord1.lat * p).cos()
//             * (coord2.lat * p).cos()
//             * (1f32 - ((coord2.lon - coord1.lon) * p).cos())
//             / 2f32;

//     a.sqrt().asin() * EARTH_RADIUS // 2 * R; R = 6371 km
// }

fn path_length_m(path: &[Coords2D]) -> f32 {
    path.windows(2).fold(0f32, |acc, cur| {
        acc + great_circle_distance(&cur[0], &cur[1])
    })
}

fn path_waypoints(path: &[Coords2D]) -> Vec<f32> {
    path.windows(2)
        .scan(0f32, |mut state, cur| {
            let out = Some(*state);

            *state = *state + great_circle_distance(&cur[0], &cur[1]);

            out
        })
        .collect()

    // path.windows(2).for_each(|(left,right)| {

    // })
}

#[derive(Debug)]
struct Path {
    pub points: Vec<PathPoint>,
    len: f32,
}
impl Path {
    fn new_from_coords(coordinates: &[Coords2D]) -> Self {
        let len = path_length_m(coordinates);

        let mut points: Vec<PathPoint> = coordinates
            .iter()
            .map(|p| PathPoint {
                coordinates: p.clone(),
                start_offset: 0f32,
            })
            .collect();

        let mut dist: f32 = 0f32;

        coordinates
            .windows(2)
            .enumerate()
            .for_each(|(index, coords)| {
                points.get_mut(index).unwrap().start_offset = dist;

                dist += great_circle_distance(&coords[0], &coords[1]);
            });

        Path { len, points }
    }

    pub fn len(&self) -> f32 {
        self.len
    }
}

#[derive(Debug)]
struct PathPoint {
    coordinates: Coords2D,
    start_offset: f32,
}

impl Link {
    fn new_from_json_link(json: &JsonLink) -> Self {
        Link {
            from: json.properties.from.clone(),
            to: json.properties.to.clone(),
            path: Path::new_from_coords(&json.geometry.coordinates),
        }
    }
}

#[derive(Deserialize, Debug)]
struct Properties {
    from: String,
    to: String,
}

#[derive(Deserialize, Debug)]
struct JsonLink {
    geometry: Geometry,
    properties: Properties,
}

#[derive(Deserialize, Debug)]
struct Geometry {
    coordinates: Vec<Coords2D>,
}

pub struct DataRepo {
    links: Vec<Link>,
    stations: Vec<stations::Station>,
}

fn extract_links(file: &File) -> Vec<Link> {
    let reader = BufReader::new(file);
    let mut json: serde_json::Value = serde_json::from_reader(reader).expect("valid parse");

    let mut json = json
        .as_object_mut()
        .unwrap()
        .get_mut("payload")
        .unwrap()
        .take();

    let json = json
        .as_object_mut()
        .unwrap()
        .get_mut("features")
        .unwrap()
        .take();

    // println!("{:?}", json);

    // println!("{}", json.is_array());
    // println!("{}", json.get(0).unwrap().is_object());

    let links: Vec<JsonLink> = serde_json::from_value(json)
        // .map_err(|e| format!("{} {} {:?}", e.line(), e.column(), e.classify()))
        .unwrap();

    links
        .into_iter()
        .map(|l| Link::new_from_json_link(&l))
        .collect()
}

impl DataRepo {
    pub fn new(cache_dir: &std::path::Path) -> Self {
        let iff_file = File::open(cache_dir.join("ns-latest.zip")).expect("To find timetable file");
        let route_file = File::open(cache_dir.join("route.json")).expect("To find route file");
        let stations_file =
            File::open(cache_dir.join("stations.json")).expect("To find stations file");

        let timetable = iff::parsing::Iff::from_file(&iff_file).unwrap();

        let links = extract_links(&route_file);
        let stations = extract_stations(&stations_file);

        println!("{:?}", stations);

        DataRepo { links, stations }
    }
}
