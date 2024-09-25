use std::{
    collections::HashMap,
    fmt::{Display, Write},
    fs::File,
    io::{self, Cursor, Read},
    str::FromStr,
};

use chrono::NaiveDate;
use parsing::{
    parse_company_file, parse_delivery_file, parse_footnote_file, parse_timetable_file, CompanyFile,
};
use serde::Serialize;
use winnow::{BStr, Parser};

use crate::dayoffset::DayOffset;

use self::parsing::TransitMode;

mod parsing;

const FOOTNOTE_FILE_NAME: &str = "footnote.dat";
const TIMETABLE_FILE_NAME: &str = "timetbls.dat";
const COMPANY_FILE_NAME: &str = "company.dat";
const HEADER_FILENAME: &str = "delivery.dat";

pub struct Iff {
    timetable: TimeTable,
    validity: RideValidity,
    companies: Vec<Company>,
    header: Header,
    pub locations: LocationCache,
}

impl Iff {
    pub fn new_from_archive(archive: &File) -> Result<Self, String> {
        let (timetable, locations) = Self::parse_timetable(archive)?;
        let validity = Self::parse_validity(archive)?;
        let companies = Self::parse_companies(archive).map(|c| c.companies)?;
        let delivery = Self::parse_delivery(archive)?;

        Ok(Self {
            locations,
            timetable,
            validity,
            companies,
            header: delivery,
        })
    }

    pub fn timetable(&self) -> &TimeTable {
        &self.timetable
    }

    pub fn timetable_mut(&mut self) -> &mut TimeTable {
        &mut self.timetable
    }

    pub fn rides_mut(&mut self) -> &mut Vec<Record> {
        &mut self.timetable.rides
    }

    pub fn validity(&self) -> &RideValidity {
        &self.validity
    }

    pub fn companies(&self) -> &[Company] {
        &self.companies
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    fn parse_timetable(
        archive: impl Read + io::Seek,
    ) -> Result<(TimeTable, LocationCache), String> {
        let content = read_bytes_from_archive(archive, TIMETABLE_FILE_NAME)?;
        let content = BStr::new(&content);
        if !content.is_ascii() {
            return Err("Expected timetable file to be valid ASCII".to_owned());
        }

        parse_timetable_file
            .parse(content)
            .map_err(|o| o.to_string())
    }

    fn parse_validity(archive: impl Read + io::Seek) -> Result<RideValidity, String> {
        let content = read_string_from_archive(archive, FOOTNOTE_FILE_NAME)?;
        let content = BStr::new(&content);
        if !content.is_ascii() {
            return Err("Expected timetable file to be valid ASCII".to_owned());
        }

        parse_footnote_file
            .parse(content)
            .map_err(|o| o.to_string())
    }

    fn parse_companies(archive: impl Read + io::Seek) -> Result<CompanyFile, String> {
        let content = read_string_from_archive(archive, COMPANY_FILE_NAME)?;
        let content = BStr::new(&content);
        if !content.is_ascii() {
            return Err("Expected timetable file to be valid ASCII".to_owned());
        }

        parse_company_file(content).map_err(|o| o.to_string())
    }

    pub fn parse_delivery(archive: impl Read + io::Seek) -> Result<Header, String> {
        let content = read_string_from_archive(archive, HEADER_FILENAME)?;
        let content = BStr::new(&content);
        if !content.is_ascii() {
            return Err("Expected timetable file to be valid ASCII".to_owned());
        }

        parse_delivery_file(content).map_err(|o| o.to_string())
    }

    pub fn parse_version_only(data: &[u8]) -> Result<u64, String> {
        let cursor = Cursor::new(data);
        let content = read_bytes_from_archive(cursor, HEADER_FILENAME)?;
        let content = BStr::new(&content);
        if !content.is_ascii() {
            return Err("Expected timetable file to be valid ASCII".to_owned());
        }

        parse_delivery_file(content)
            .map_err(|e| e.to_string())
            .map(|h| h.version)
    }
}

fn read_string_from_archive(
    archive: impl Read + io::Seek,
    filename: &str,
) -> Result<String, String> {
    let mut archive = zip::ZipArchive::new(archive).expect("valid new archive");
    let mut file = archive
        .by_name(filename)
        .map_err(|_| "Error getting file from archive")?;

    let mut buf = vec![];

    file.read_to_end(&mut buf).map_err(|e| e.to_string())?;

    // File should be ISO 8859-1 / Latin1, this should work fine
    let str_content = String::from_utf8(buf).map_err(|_| "file contained invalid utf-8")?;

    Ok(str_content.to_owned())
}

fn read_bytes_from_archive(
    archive: impl Read + io::Seek,
    filename: &str,
) -> Result<Vec<u8>, String> {
    let mut archive = zip::ZipArchive::new(archive).expect("valid new archive");
    let mut file = archive
        .by_name(filename)
        .map_err(|_| "Error getting file from archive")?;

    let mut buf = vec![];
    file.read_to_end(&mut buf).map_err(|e| e.to_string())?;

    if !buf.is_ascii() {
        return Err("File is not valid ASCII".into());
    }

    Ok(buf)
}

#[derive(PartialEq, Debug, Eq, Clone, Serialize)]
pub struct TimetableEntry {
    pub code: LocationCodeHandle,
    pub stop_kind: StopKind,
}

impl TimetableEntry {
    fn serializable<'a, 'b>(&'a self, cache: &'b LocationCache) -> TimetableEntryContext
    where
        'b: 'a,
    {
        TimetableEntryContext {
            entry: self,
            context: cache,
        }
    }
}

pub struct TimetableEntryContext<'e, 'c> {
    pub entry: &'e TimetableEntry,
    pub context: &'c LocationCache,
}

impl<'e, 'c> Serialize for TimetableEntryContext<'e, 'c> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let code = self.context.get_str(&self.entry.code).unwrap();

        TimetableEntryRaw {
            stop_kind: self.entry.stop_kind.clone(),
            code,
        }
        .serialize(serializer)
    }
}

#[derive(Serialize)]
struct TimetableEntryRaw<'a> {
    pub code: &'a str,
    pub stop_kind: StopKind,
}

impl<'a> TimetableEntryRaw<'a> {
    pub fn to_proper(&self, cache: &mut LocationCache) -> TimetableEntry {
        TimetableEntry {
            code: cache.get_handle(self.code),
            stop_kind: self.stop_kind.clone(),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Eq)]
pub struct Platform {
    suffix: Option<char>,
    number: u8,
    range_to: Option<u8>,
}

impl Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.range_to {
            Some(to) => f.write_fmt(format_args!("{}-{}", self.number, to)),
            None => match self.suffix {
                Some(suffix) => f.write_fmt(format_args!("{}{}", self.number, suffix)),
                None => f.write_fmt(format_args!("{}", self.number)),
            },
        }
    }
}

impl Serialize for Platform {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut output = String::new();
        output.push_str(&self.number.to_string());
        if let Some(suffix) = self.suffix {
            output.push(suffix);
        } else if let Some(range) = self.range_to {
            output.push('-');
            output.push_str(&range.to_string())
        }

        serializer.serialize_str(&output)
    }
}

impl Platform {
    pub fn plain(n: u8) -> Self {
        Platform {
            number: n,
            suffix: None,
            range_to: None,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Eq, Serialize)]
pub struct PlatformParseError;

impl FromStr for Platform {
    type Err = PlatformParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        // Common case
        let num_parse_result = u8::from_str(s);
        if let Ok(number) = num_parse_result {
            return Ok(Platform {
                number,
                suffix: None,
                range_to: None,
            });
        }

        if s.ends_with(|c: char| c.is_alphabetic()) {
            let a = &s[0..s.len() - 1];

            return Ok(Platform {
                number: a.parse().unwrap(), // todo not unwrap
                suffix: s.chars().last(),
                range_to: None,
            });
        }

        if s.is_empty() {
            return Err(PlatformParseError {});
        }

        // Special case
        if s.chars().nth(1).expect("Expected to find a -") == '-' {
            let split = s.split_once('-').expect("clean platform split");

            let left: u8 = split
                .0
                .parse()
                .expect("expected first number parse to succeed");
            let right: u8 = split
                .1
                .parse()
                .expect("expected first number parse to succeed");

            return Ok(Platform {
                number: left,
                suffix: None,
                range_to: Some(right),
            });
        }

        Err(PlatformParseError)
        // common case
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct LocationCodeHandle {
    inner: u16,
}

#[derive(Clone, Debug, Serialize)]
#[serde(transparent)]
pub struct LocationCache {
    storage: Vec<Box<str>>,
    #[serde(skip)]
    lookup: HashMap<Box<str>, u16>,
}

impl LocationCache {
    pub fn new() -> Self {
        Self {
            lookup: HashMap::new(),
            storage: Vec::new(),
        }
    }

    pub fn with_capacity(size: usize) -> Self {
        Self {
            lookup: HashMap::with_capacity(size),
            storage: Vec::with_capacity(size),
        }
    }

    pub fn get_handle(&mut self, code: &str) -> LocationCodeHandle {
        if let Some(index) = self.lookup.get(code) {
            LocationCodeHandle { inner: *index }
        } else {
            self.storage.push(code.into());

            // Index of the storage entry we just pushed
            let latest_index = self.storage.len() - 1;
            self.lookup.insert(code.into(), latest_index as u16);

            LocationCodeHandle {
                inner: latest_index as u16,
            }
        }
    }

    pub fn lookup_handle(&self, code: &str) -> Option<LocationCodeHandle> {
        self.lookup
            .get(code)
            .map(|idx| LocationCodeHandle { inner: *idx })
    }

    pub fn get_str(&self, h: &LocationCodeHandle) -> Option<&str> {
        self.storage.get(h.inner as usize).map(|bx| bx.as_ref())
    }

    pub fn codes(&self) -> &[Box<str>] {
        &self.storage
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
pub struct PlatformInfo {
    pub arrival_platform: Option<Platform>,
    pub departure_platform: Option<Platform>,
    footnote: u64,
}

impl PlatformInfo {
    pub fn plain(number: u8, footnote: u64) -> Self {
        PlatformInfo {
            arrival_platform: Some(Platform::plain(number)),
            departure_platform: Some(Platform::plain(number)),
            footnote,
        }
    }
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
        #[allow(clippy::match_same_arms)]
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

    // If passengers can board the train at this stop
    pub fn is_boardable(&self) -> bool {
        match self {
            StopKind::Departure(_, _) => true,
            StopKind::Arrival(_, _) => false,
            StopKind::Waypoint => false,
            StopKind::StopShort(_, _) => true,
            StopKind::StopLong(_, _, _) => true,
        }
    }

    pub fn platform_info(&self) -> Option<&PlatformInfo> {
        match self {
            StopKind::Departure(pl, _) => pl.as_ref(),
            StopKind::Arrival(pl, _) => pl.as_ref(),
            StopKind::Waypoint => None,
            StopKind::StopShort(pl, _) => pl.as_ref(),
            StopKind::StopLong(pl, _, _) => pl.as_ref(),
        }
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
    // pub locations: LocationCache,
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

#[derive(PartialEq, Debug, Eq, Clone)]
pub struct Record {
    pub id: u64,
    pub timetable: Vec<TimetableEntry>,
    pub ride_id: Vec<RideId>,
    pub day_validity_footnote: u64,
    pub transit_types: Vec<TransitMode>,
}

#[derive(PartialEq, Debug, Eq, Clone, Serialize)]
pub struct Footnote {
    pub footnote: u64,
    pub first_stop: u64,
    pub last_stop: u64,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct Ride {
    pub id: String,
    pub transit_mode: String,
    pub timetable: Vec<TimetableEntry>,
    pub day_validity: u64,
    pub previous: Option<String>,
    pub next: Option<String>,
    pub operator: u32,
}

pub struct RidePrettyPrint<'a>(&'a Ride, &'a LocationCache);

impl<'a> Display for RidePrettyPrint<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char('#')?;
        f.write_str(&self.0.id)?;
        for stop in &self.0.timetable {
            let code = self.1.get_str(&stop.code).unwrap();
            f.write_str(code)?;
            f.write_char('\n')?;
        }
        f.write_char('\n')
    }
}

impl Ride {
    pub fn stop_at_code(&self, code: &LocationCodeHandle) -> Option<&TimetableEntry> {
        self.timetable
            .iter()
            .find(|entry| entry.code == *code && !entry.stop_kind.is_waypoint())
    }
    // TODO This needs to take footnotes into account for special trains eg international
    pub fn boardable_at_code(&self, code: &LocationCodeHandle) -> bool {
        self.timetable
            .iter()
            .any(|entry| entry.code == *code && entry.stop_kind.is_boardable())
    }

    pub fn pretty_print<'a>(&'a self, codes: &'a LocationCache) -> RidePrettyPrint<'a> {
        RidePrettyPrint(self, codes)
    }
}

#[derive(Debug)]
pub enum LegKind {
    Stationary(LocationCodeHandle, StopKind),
    Moving {
        from: LocationCodeHandle,
        to: LocationCodeHandle,
        waypoints: Vec<LocationCodeHandle>,
    },
}

impl LegKind {
    pub fn is_moving(&self) -> bool {
        matches!(
            &self,
            Self::Moving {
                from: _,
                to: _,
                waypoints: _
            }
        )
    }

    pub fn waypoints(&self) -> Option<&Vec<LocationCodeHandle>> {
        match self {
            Self::Stationary(_, _) => None,
            Self::Moving {
                from: _,
                to: _,
                waypoints: wp,
            } => Some(wp),
        }
    }

    pub fn from(&self) -> Option<&LocationCodeHandle> {
        match self {
            Self::Stationary(_, _) => None,
            Self::Moving {
                from,
                to: _,
                waypoints: _,
            } => Some(from),
        }
    }

    pub fn to(&self) -> Option<&LocationCodeHandle> {
        match self {
            Self::Stationary(_, _) => None,
            Self::Moving {
                from: _,
                to,
                waypoints: _,
            } => Some(to),
        }
    }

    pub fn station_code(&self) -> Option<&LocationCodeHandle> {
        match self {
            Self::Stationary(code, _) => Some(code),
            Self::Moving {
                from: _,
                to: _,
                waypoints: _,
            } => None,
        }
    }

    pub fn platform_info(&self) -> Option<&PlatformInfo> {
        #[allow(clippy::match_same_arms)]
        match self {
            Self::Stationary(_, kind) => match kind {
                StopKind::Departure(plat, _) => plat.as_ref(),
                StopKind::Arrival(plat, _) => plat.as_ref(),
                StopKind::Waypoint => None,
                StopKind::StopShort(plat, _) => plat.as_ref(),
                StopKind::StopLong(plat, _, _) => plat.as_ref(),
            },
            Self::Moving {
                from: _,
                to: _,
                waypoints: _,
            } => None,
        }
    }
}

#[derive(Debug)]
pub struct Leg {
    pub start: DayOffset,
    pub end: DayOffset,
    // #[serde(flatten)]
    pub kind: LegKind,
}

#[derive(Serialize, Debug)]
pub struct Company {
    id: u32,
    code: Box<str>,
    name: Box<str>,
    end_of_timetable: DayOffset,
}
