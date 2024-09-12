use std::{fs::File, io::BufReader};

use serde::{Deserialize, Serialize};

use crate::iff::{LocationCache, LocationCodeHandle};

use super::LinkCode;

pub fn extract_links(file: &File, locations: &mut LocationCache) -> Vec<Link> {
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
        .map(|l| Link::new_from_json_link(&l, locations))
        .collect()
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
pub struct Coords2D {
    // #[serde(serialize_with = "bin_float")]
    longitude: f64, // Do not change the order, matters for Deserialize as it's parsing an array of 2 numbers into this struct
    // #[serde(serialize_with = "bin_float")]
    latitude: f64,
}

impl Eq for Coords2D {}

impl Coords2D {
    pub fn new(longitude: f64, latitude: f64) -> Self {
        // longitude.
        Self {
            longitude,
            latitude,
        }
    }
}

/// A path between two timetable points
#[derive(Debug, Clone, Serialize)]
pub struct Link {
    // id: u32,
    from: LocationCodeHandle,
    to: LocationCodeHandle,
    path: Path,
}

// struct LinkSerializable<'a, 'b> {
//     inner: &'a Link,
//     location_cache: &'b LocationCache,
// }

impl PartialEq for Link {
    fn eq(&self, other: &Self) -> bool {
        self.from == other.from && self.to == other.to
    }
}

impl Link {
    pub fn link_code(&self) -> LinkCode {
        LinkCode(self.from, self.to)
    }
}

// const EARTH_RADIUS: f32 = 12742f32;

// https://stackoverflow.com/a/21623206
// fn greatCircleDistance(lat1, lon1, lat2, lon2 float64) float64 {
fn great_circle_distance(coords1: &Coords2D, coords2: &Coords2D) -> f64 {
    let radius: f64 = 6371f64; // km
    let p: f64 = std::f64::consts::PI / 180f64;

    let a = 0.5f64 - ((coords2.latitude - coords1.latitude) * p).cos() / 2f64
        + (coords1.latitude * p).cos()
            * (coords2.latitude * p).cos()
            * (1f64 - ((coords2.longitude - coords1.longitude) * p).cos())
            / 2f64;

    2f64 * radius * a.sqrt().asin()

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

// fn path_length_m(path: &[Coords2D]) -> f64 {
//     path.windows(2).fold(0f64, |acc, cur| {
//         acc + great_circle_distance(&cur[0], &cur[1])
//     })
// }

// fn path_waypoints(path: &[Coords2D]) -> Vec<f64> {
//     path.windows(2)
//         .scan(0f64, |state, cur| {
//             let out = Some(*state);

//             *state += great_circle_distance(&cur[0], &cur[1]);

//             out
//         })
//         .collect()

//     // path.windows(2).for_each(|(left,right)| {

//     // })
// }

// fn bin_float<S>(f: &f64, s: S) -> Result<S::Ok, S::Error>
// where
//     S: Serializer,
// {
//     s.serialize_bytes((*f as f32).to_le_bytes().as_slice())
// }

/// A point on a Path
#[derive(Debug, Serialize, Clone)]
struct PathPoint {
    coordinates: Coords2D,
    #[serde(skip_serializing)]
    start_offset: f64,
}

impl Link {
    fn new_from_json_link(json: &JsonLink, location_cache: &mut LocationCache) -> Self {
        let from = location_cache.get_handle(&json.properties.from);
        let to = location_cache.get_handle(&json.properties.to);

        Self {
            from,
            to,
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

/// A linear path formed of a series of `PathPoints`
#[derive(Debug, Serialize, Clone)]
struct Path {
    pub points: Vec<PathPoint>,
    #[serde(skip_serializing)]
    _len: f64,
}

impl Path {
    fn new_from_coords(coordinates: &[Coords2D]) -> Self {
        let distances: Vec<(usize, f64)> = coordinates
            .windows(2)
            .map(|slice| great_circle_distance(&slice[0], &slice[1]))
            .enumerate()
            .collect();

        // for a in coordinates {
        //     println!("{:?}", a);
        // }
        // println!("{:?}", distances);

        // let mut dist: f32 = 0f32;

        // points
        //     .iter()
        //     .enumerate()
        //     .skip(1)
        //     .for_each(|(index, current)| {
        //         let previous = points.get(index - 1).unwrap(); // As long as we skip the first entry this is safe
        //         let current_distance = previous.start_offset
        //             + great_circle_distance(&current.coordinates, &previous.coordinates);

        //         points.get_mut(index).unwrap().start_offset = current_distance
        //     });

        let mut points: Vec<PathPoint> = coordinates
            .iter()
            .map(|coords| PathPoint {
                coordinates: *coords,
                start_offset: 0f64,
            })
            .collect();

        let mut sum = 0f64;
        for distance in distances {
            sum += distance.1;
            points.get_mut(distance.0 + 1).unwrap().start_offset = sum;

            // println!("Setting index {} to {}", distance.0 + 1, sum);
        }

        // coordinates
        //     .windows(2)
        //     .enumerate()
        //     .for_each(|(index, coords)| {
        //         points.get_mut(index).unwrap().start_offset = dist;

        //         dist += great_circle_distance(&coords[0], &coords[1]);
        //     });

        // let total_length = path_length_m(coordinates);
        Self { _len: sum, points }
    }

    // pub fn len(&self) -> f64 {
    //     self.len
    // }
}
