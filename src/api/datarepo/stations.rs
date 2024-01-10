use serde::{Deserialize, Serialize};
use std::{fs::File, io::BufReader};

use super::Coords2D;

#[derive(Debug, Serialize)]
pub struct Station {
    code: String,
    name: String,
    position: Coords2D,
}

impl Station {
    fn new_from_json(json: &StationJSON) -> Self {
        Self {
            code: json.code.clone(),
            name: json.namen.lang.clone(),
            position: Coords2D::new(json.lng, json.lat),
        }
    }
}

#[derive(Deserialize)]
struct NamesJSON {
    lang: String,
}

#[derive(Deserialize)]
struct StationJSON {
    code: String,
    namen: NamesJSON,
    lat: f32,
    lng: f32,
}

pub fn extract_stations(file: &File) -> Vec<Station> {
    let reader = BufReader::new(file);
    let mut json: serde_json::Value = serde_json::from_reader(reader).expect("valid parse");

    let mut json = json
        .as_object_mut()
        .unwrap()
        .get_mut("payload")
        .unwrap()
        .take();
    assert!(json.is_array());

    let stations: Vec<StationJSON> = serde_json::from_value(json).unwrap();

    stations
        .into_iter()
        .map(|s| Station::new_from_json(&s))
        .collect()
}
