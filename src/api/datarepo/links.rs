use ordered_float::{Float, OrderedFloat};
use serde::{Deserialize, Serialize};
use std::{fs::File, io::BufReader};

use super::{stations::Station, LinkCode};

pub fn extract_links(file: &File) -> Vec<Link> {
    let reader = BufReader::new(file);

    // Parse into serde_json::Value first to easily navigate down the json data
    let mut json: serde_json::Value = serde_json::from_reader(reader).expect("valid parse");

    let mut json = json
        .as_object_mut()
        .expect("top level to be an object")
        .get_mut("payload")
        .expect("a 'payload' member object")
        .take();

    let json = json
        .as_object_mut()
        .expect("top level to be an object")
        .get_mut("features")
        .expect("a 'features' member object")
        .take();

    let links: Vec<JsonLink> =
        serde_json::from_value(json).expect("should parse JsonLinks from raw link json");

    links
        .into_iter()
        .map(|l| Link::new_from_json_link(&l))
        .collect()
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Coords2D {
    // #[serde(serialize_with = "bin_float")]
    longitude: OrderedFloat<f64>, // Do not change the order, matters for Deserialize as it's parsing an array of 2 numbers into this struct
    // #[serde(serialize_with = "bin_float")]
    latitude: OrderedFloat<f64>,
}

impl Coords2D {
    pub fn new(longitude: f64, latitude: f64) -> Self {
        debug_assert!(longitude.is_finite());
        debug_assert!(latitude.is_finite());
        Self {
            longitude: OrderedFloat(longitude),
            latitude: OrderedFloat(latitude),
        }
    }
}

/// A path between two timetable points
#[derive(Debug, Serialize, Clone)]
pub struct Link {
    // id: u32,
    from: Station,
    to: Station,
    path: Path,
}

impl PartialEq for Link {
    fn eq(&self, other: &Self) -> bool {
        self.from == other.from && self.to == other.to
    }
}

impl Link {
    pub fn link_code(&self) -> LinkCode {
        LinkCode(self.from.clone(), self.to.clone())
    }
}

// const EARTH_RADIUS: f32 = 12742f32;

// https://stackoverflow.com/a/21623206
// fn greatCircleDistance(lat1, lon1, lat2, lon2 float64) float64 {
fn great_circle_distance(coords1: &Coords2D, coords2: &Coords2D) -> f64 {
    let radius: f64 = 6371f64; // km
    let p: f64 = std::f64::consts::PI / 180f64;

    let a = OrderedFloat(0.5f64) - ((coords2.latitude - coords1.latitude) * p).cos() / 2f64
        + (coords1.latitude * p).cos()
            * (coords2.latitude * p).cos()
            * (OrderedFloat(1f64) - ((coords2.longitude - coords1.longitude) * p).cos())
            / OrderedFloat(2f64);

    (OrderedFloat(2f64) * radius * a.sqrt().asin()).into_inner()
}

impl Link {
    fn new_from_json_link(json: &JsonLink) -> Self {
        Self {
            from: Station::from_code(&json.properties.from),
            to: Station::from_code(&json.properties.to),
            path: Path::new_from_coords(&json.geometry.coordinates),
        }
    }
}

#[derive(Debug, Serialize, Clone)]
struct PathPoint {
    coordinates: Coords2D,
    #[serde(skip_serializing)]
    start_offset: f64,
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

/// A linear path formed of a series of `PathPoints`
#[derive(Debug, Serialize, Clone)]
struct Path {
    pub points: Vec<PathPoint>,
    #[serde(skip_serializing)]
    _len: f64,
}

impl Path {
    fn new_from_coords(coordinates: &[Coords2D]) -> Self {
        let distances = coordinates
            .windows(2)
            .map(|slice| great_circle_distance(&slice[0], &slice[1]));

        let mut points: Vec<PathPoint> = coordinates
            .iter()
            .map(|coords| PathPoint {
                coordinates: *coords,
                start_offset: 0f64,
            })
            .collect();

        let mut sum = 0f64;
        for (index, distance) in distances.enumerate() {
            sum += distance;
            points.get_mut(index + 1).unwrap().start_offset = sum;
        }

        Self { _len: sum, points }
    }
}
