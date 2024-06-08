use serde::{Deserialize, Serialize, Serializer};
use std::{fs::File, io::BufReader, str::FromStr};

use super::links::Coords2D;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Station {
    pub code: String,
    pub name: String,
    pub position: Coords2D,
    // #[serde(serialize_with = "path")]
    #[serde(serialize_with = "serialize_station_to_rank")]
    #[serde(rename(serialize = "rank"))]
    pub station_type: StationType,
}

impl Station {
    fn new_from_json(json: &StationJSON) -> Self {
        Self {
            code: json.code.to_lowercase(),
            name: json.namen.lang.clone(),
            position: Coords2D::new(json.lng, json.lat),
            station_type: StationType::from_str(&json.stationType).unwrap(),
        }
    }
}

#[derive(Deserialize)]
struct NamesJSON {
    //Long
    lang: String,

    // Medium
    middel: String,

    // Short
    kort: String,
}

#[derive(Deserialize)]
struct StationJSON {
    code: String,
    namen: NamesJSON,
    lat: f64,
    lng: f64,
    #[allow(non_snake_case)]
    stationType: String,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub enum StationType {
    Mega,
    InterCityTransfer,
    InterCity,
    ExpressTransfer,
    Express,
    LocalTransfer,
    Local,
    Technical,
}

fn serialize_station_to_rank<S: Serializer>(
    station_type: &StationType,
    ser: S,
) -> Result<S::Ok, S::Error> {
    Ok(ser.serialize_u8(station_type.as_rank())?)
}

// fn<S>(&T, S) -> Result<S::Ok, S::Error> where S: Serializer

impl StationType {
    fn as_rank(&self) -> u8 {
        match self {
            StationType::Mega => 7,
            StationType::InterCityTransfer => 6,
            StationType::InterCity => 5,
            StationType::ExpressTransfer => 4,
            StationType::Express => 3,
            StationType::LocalTransfer => 2,
            StationType::Local => 1,
            StationType::Technical => 0,
        }
    }

    fn is_transfer(&self) -> bool {
        match self {
            StationType::Mega => true,
            StationType::InterCityTransfer => true,
            StationType::InterCity => false,
            StationType::ExpressTransfer => true,
            StationType::Express => false,
            StationType::LocalTransfer => true,
            StationType::Local => false,
            StationType::Technical => false,
        }
    }
}

impl FromStr for StationType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mega_station" => Ok(Self::Mega),
            "knooppunt_intercity_station" => Ok(Self::InterCityTransfer),
            "intercity_station" => Ok(Self::InterCity),
            "knooppunt_sneltrein_station" => Ok(Self::ExpressTransfer),
            "sneltrein_station" => Ok(Self::Express),
            "knooppunt_stoptrein_station" => Ok(Self::LocalTransfer),
            "stoptrein_station" => Ok(Self::Local),
            "facultatief_station" => Ok(Self::Technical),
            _ => Err(()),
        }
    }
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

    stations
        .into_iter()
        .map(|s| Station::new_from_json(&s))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parse() {
        let test_data = include_str!("./testdata/station.json");
        let station: Station = serde_json::from_str::<StationJSON>(test_data)
            .map(|s| Station::new_from_json(&s))
            .unwrap();

        assert_eq!(
            station,
            Station {
                code: String::from("gp"),
                name: String::from("Geldrop"),
                position: Coords2D::new(5.55055570602417, 51.4197235107422),
                station_type: StationType::LocalTransfer
            }
        )
    }
}
