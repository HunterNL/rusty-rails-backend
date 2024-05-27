use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    hash::Hash,
    io::BufReader,
    sync::{Arc, RwLock},
};

use super::links::Coords2D;

#[derive(Debug, Clone, Serialize)]
pub struct Station(Arc<InnerStation>);

impl Station {
    pub fn code(&self) -> String {
        self.0.code.clone()
    }
    pub fn name(&self) -> String {
        self.0.name.clone()
    }
    pub fn position(&self) -> Coords2D {
        self.0.position
    }
}

impl PartialEq for Station {
    fn eq(&self, other: &Self) -> bool {
        *(self.0) == *(other.0)
    }
}

impl Hash for Station {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl Eq for Station {}

#[derive(Debug, Serialize, Hash, PartialEq, Eq)]
pub struct InnerStation {
    pub code: String,
    pub name: String,
    pub position: Coords2D,
}

lazy_static::lazy_static! {
    static ref KNOWN_STATIONS: RwLock<HashMap<String, Arc<InnerStation>>> = RwLock::new(HashMap::default());
}

impl Station {
    fn new_from_json(json: StationJSON) -> Station {
        let code = json.code.to_lowercase();

        if let Some(arc) = KNOWN_STATIONS.read().unwrap().get(&code) {
            Station(arc.clone())
        } else {
            let arc = Arc::new(InnerStation {
                code: code.clone(),
                name: json.namen.lang.clone(),
                position: Coords2D::new(json.lng, json.lat),
            });
            let inner = KNOWN_STATIONS
                .write()
                .unwrap()
                .entry(code)
                .or_insert(arc)
                .clone();
            Station(inner)
        }
    }

    pub fn from_code(code: &str) -> Station {
        Station(
            KNOWN_STATIONS
                .read()
                .unwrap()
                .get(code)
                .expect("Tried to find unknown station")
                .clone(),
        )
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
    lat: f64,
    lng: f64,
}

pub fn extract_stations(file: &File) -> Vec<Station> {
    let reader = BufReader::new(file);
    let mut json: serde_json::Value = serde_json::from_reader(reader).expect("valid parse");

    let json = json
        .as_object_mut()
        .unwrap()
        .get_mut("payload")
        .unwrap()
        .take();
    assert!(json.is_array());

    let stations: Vec<StationJSON> = serde_json::from_value(json).unwrap();

    stations.into_iter().map(Station::new_from_json).collect()
}
