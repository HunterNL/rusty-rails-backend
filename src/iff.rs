use std::{collections::HashMap, fs::File, io::Read};

use chrono::NaiveDate;
use serde::Serialize;
use winnow::Parser;

use crate::dayoffset::DayOffset;

use self::parsing::{parse_footnote_file, parse_timetable_file, TransitMode};

mod parsing;

const FOOTNOTE_FILE_NAME: &str = "footnote.dat";
const TIMETABLE_FILE_NAME: &str = "timetbls.dat";

pub struct Iff {
    timetable: TimeTable,
    validity: RideValidity,
}

impl Iff {
    pub fn new_from_archive(archive: &File) -> Result<Self, String> {
        let timetable = Self::parse_timetable(archive)?;
        let validity = Self::parse_validity(archive)?;

        Ok(Self {
            timetable,
            validity,
        })
    }

    pub fn timetable(&self) -> &TimeTable {
        &self.timetable
    }

    pub fn timetable_mut(&mut self) -> &mut TimeTable {
        &mut self.timetable
    }

    pub fn validity(&self) -> &RideValidity {
        &self.validity
    }

    fn parse_timetable(archive: &File) -> Result<TimeTable, String> {
        let content = read_file_from_archive(archive, TIMETABLE_FILE_NAME)?;

        parse_timetable_file
            .parse(&content)
            .map_err(|o| o.to_string())
    }

    fn parse_validity(archive: &File) -> Result<RideValidity, String> {
        let content = read_file_from_archive(archive, FOOTNOTE_FILE_NAME)?;

        parse_footnote_file
            .parse(&content)
            .map_err(|o| o.to_string())
    }
}

fn read_file_from_archive(archive: &File, filename: &str) -> Result<String, String> {
    let mut archive = zip::ZipArchive::new(archive).expect("valid new archive");
    let mut file = archive
        .by_name(filename)
        .map_err(|_| "Error getting file from archive")?;

    let mut buf = vec![];

    file.read_to_end(&mut buf).map_err(|e| e.to_string())?;

    // File should be ISO 8859-1 / Latin1, this should work fine
    let str_content =
        std::str::from_utf8(buf.as_slice()).map_err(|_| "file contained invalid utf-8")?;

    Ok(str_content.to_owned())
}

#[derive(PartialEq, Debug, Eq, Clone, Serialize)]
pub struct TimetableEntry {
    pub code: String,
    pub stop_kind: StopKind,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
pub struct PlatformInfo {
    arrival_platform: Option<String>,
    departure_platform: Option<String>,
    footnote: u64,
}

#[derive(PartialEq, Debug, Eq, Clone, Serialize)]
pub enum StopKind {
    Departure(Option<PlatformInfo>, DayOffset),
    Arrival(Option<PlatformInfo>, DayOffset),
    Waypoint,
    StopShort(Option<PlatformInfo>, DayOffset),
    StopLong(Option<PlatformInfo>, DayOffset, DayOffset),
}

impl StopKind {
    pub fn departure_time(&self) -> Option<&DayOffset> {
        match self {
            Self::Departure(_, departure_time) => Some(departure_time),
            Self::Arrival(_, _) | Self::Waypoint => None,
            Self::StopShort(_, time) => Some(time),
            Self::StopLong(_, _, departure_time) => Some(departure_time),
        }
    }

    pub fn arrival_time(&self) -> Option<&DayOffset> {
        match self {
            Self::Departure(_, _) => None,
            Self::Arrival(_, arrival_time) => Some(arrival_time),
            Self::Waypoint => None,
            Self::StopShort(_, time) => Some(time),
            Self::StopLong(_, arrival_time, _) => Some(arrival_time),
        }
    }

    pub fn is_waypoint(&self) -> bool {
        self == &Self::Waypoint
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Header {
    pub company_id: u64,
    pub first_valid_date: chrono::NaiveDate,
    pub last_valid_date: chrono::NaiveDate,
    pub version: u64,
    pub description: String,
}

struct DayValidityFootnote {
    id: u64,
    validity: Vec<bool>,
}

pub struct TimeTable {
    pub header: Header,
    pub rides: Vec<Record>,
}

pub struct RideValidity {
    header: Header,
    validities: HashMap<u64, Vec<bool>>,
}

impl RideValidity {
    pub fn is_valid_on_day(&self, footnote_id: u64, date: NaiveDate) -> Result<bool, ()> {
        if date < self.header.first_valid_date || date > self.header.last_valid_date {
            return Err(()); // Out of validity range
        }

        //TODO Investigate, Might be off by one
        let day_id = date
            .signed_duration_since(self.header.first_valid_date)
            .num_days() as u64;

        self.validities.get(&footnote_id).ok_or(()).map(|v| {
            *v.get(day_id as usize)
                .expect("to find footnote in validity lookup")
        })
    }
}

#[derive(PartialEq, Debug, Eq, Clone, Serialize)]
pub struct RideId {
    company_id: u32,
    ride_id: u32,
    line_id: Option<u32>,
    first_stop: u32,
    last_stop: u32,
    ride_name: Option<String>,
}

#[derive(PartialEq, Debug, Eq, Clone, Serialize)]
pub struct Record {
    pub id: u64,
    pub timetable: Vec<TimetableEntry>,
    pub ride_id: Vec<RideId>,
    pub day_validity_footnote: Footnote,
    pub transit_types: Vec<TransitMode>,
}

#[derive(PartialEq, Debug, Eq, Clone, Serialize)]
pub struct Footnote {
    pub footnote: u64,
    pub first_stop: u64,
    pub last_stop: u64,
}

#[derive(Serialize)]
pub enum LegKind {
    Stationary(String, StopKind),
    Moving(String, String, Vec<String>),
}

impl LegKind {
    pub fn is_moving(&self) -> bool {
        matches!(&self, LegKind::Moving(_, _, _))
    }

    pub fn waypoints(&self) -> Option<&Vec<String>> {
        match self {
            LegKind::Stationary(_, _) => None,
            LegKind::Moving(_, _, wp) => Some(wp),
        }
    }

    pub fn from(&self) -> Option<&String> {
        match self {
            LegKind::Stationary(_, _) => None,
            LegKind::Moving(from, _, _) => Some(from),
        }
    }

    pub fn to(&self) -> Option<&String> {
        match self {
            LegKind::Stationary(_, _) => None,
            LegKind::Moving(_, to, _) => Some(to),
        }
    }

    pub fn station_code(&self) -> Option<&String> {
        match self {
            LegKind::Stationary(code, _) => Some(code),
            LegKind::Moving(_, _, _) => None,
        }
    }

    pub fn platform_info(&self) -> Option<&PlatformInfo> {
        match self {
            LegKind::Stationary(_, kind) => match kind {
                StopKind::Departure(plat, _) => plat.as_ref(),
                StopKind::Arrival(plat, _) => plat.as_ref(),
                StopKind::Waypoint => None,
                StopKind::StopShort(plat, _) => plat.as_ref(),
                StopKind::StopLong(plat, _, _) => plat.as_ref(),
            },
            LegKind::Moving(_, _, _) => None,
        }
    }
}

#[derive(Serialize)]
pub struct Leg {
    pub start: DayOffset,
    pub end: DayOffset,
    #[serde(flatten)]
    pub kind: LegKind,
}
