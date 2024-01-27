use std::{fs::File, io::BufReader};

use serde::{Deserialize, Serialize};

use super::LinkCode;

pub fn extract_links(file: &File) -> Vec<Link> {
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

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
pub struct Coords2D {
    lon: f64,
    lat: f64,
}

impl Eq for Coords2D {}

impl Coords2D {
    pub fn new(lon: f64, lat: f64) -> Self {
        Self { lon, lat }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct Link {
    // id: u32,
    from: String,
    to: String,
    path: Path,
}

impl PartialEq for Link {
    fn eq(&self, other: &Self) -> bool {
        self.from == other.from && self.to == other.to
    }
}

impl Link {
    fn from(&self) -> &str {
        &self.from
    }

    fn to(&self) -> &str {
        &self.to
    }

    fn path(&self) -> &Path {
        &self.path
    }

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

    let a = 0.5f64 - ((coords2.lat - coords1.lat) * p).cos() / 2f64
        + (coords1.lat * p).cos()
            * (coords2.lat * p).cos()
            * (1f64 - ((coords2.lon - coords1.lon) * p).cos())
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

fn path_length_m(path: &[Coords2D]) -> f64 {
    path.windows(2).fold(0f64, |acc, cur| {
        acc + great_circle_distance(&cur[0], &cur[1])
    })
}

fn path_waypoints(path: &[Coords2D]) -> Vec<f64> {
    path.windows(2)
        .scan(0f64, |state, cur| {
            let out = Some(*state);

            *state = *state + great_circle_distance(&cur[0], &cur[1]);

            out
        })
        .collect()

    // path.windows(2).for_each(|(left,right)| {

    // })
}

#[derive(Debug, Serialize, Clone)]
struct PathPoint {
    coordinates: Coords2D,
    start_offset: f64,
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

#[derive(Debug, Serialize, Clone)]
struct Path {
    pub points: Vec<PathPoint>,
    len: f64,
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
        Path { len: sum, points }
    }

    pub fn len(&self) -> f64 {
        self.len
    }
}
