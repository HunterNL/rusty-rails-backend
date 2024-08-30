use chrono::NaiveDate;
use serde::Serialize;
use std::fmt::Display;
use winnow::error::ParseError;

use winnow::ascii::{dec_uint, line_ending, multispace0, Uint};
use winnow::combinator::trace;
use winnow::combinator::{alt, delimited, fail, opt};
use winnow::stream::AsChar;

use winnow::token::{take, take_till, take_until, take_while};
use winnow::{PResult, Parser};

use crate::dayoffset::DayOffset;

mod timetable;
pub use timetable::parse_footnote_file;
pub use timetable::parse_timetable_file;

mod company;
pub use company::parse_company_file;
pub use company::CompanyFile;

pub fn parse_delivery_file(
    input: &str,
) -> Result<Header, ParseError<&str, winnow::error::ContextError>> {
    parse_header.parse(input)
}

use super::{Header, Leg, LegKind, Record, Ride, StopKind, TimetableEntry};

/// Length of dates as they appear in the iff file
const DATE_FORMAT_LEN: usize = "DDMMYYYY".len();
/// Date format as required by `NaiveDate::parse_from_str`
const DATE_FORMAT: &str = "%d%m%Y";

const IFF_NEWLINE: &str = "\r\n";

pub struct InvalidEncodingError {}

impl Display for InvalidEncodingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("File was not encoded as valid UTF-8")
    }
}

// fn recognize_digits<'s>(input: &mut &'s str) -> PResult<&'s str> {
//     digit1.recognize().parse_next(input)
// }

fn seperator(input: &mut &str) -> PResult<()> {
    (multispace0, ',').void().parse_next(input)
}

fn dec_uint_leading<Output: Uint + Clone>(input: &mut &str) -> PResult<Output> {
    alt((
        (take_while(0.., '0'), dec_uint).map(|a| a.1),
        (take_while(1.., '0')).value(Output::try_from_dec_uint("0").unwrap()),
        // (take_while(1.., '0')).value(0),
        fail,
    ))
    .parse_next(input)
}

fn parse_header(input: &mut &str) -> PResult<Header> {
    // separated(1.., "100", ',').parse_peek(input)
    trace(
        "header",
        delimited(
            '@',
            (
                dec_uint_leading,
                ',',
                parse_date,
                ',',
                parse_date,
                ',',
                dec_uint_leading,
                ',',
                take_till(1.., |a: char| a.is_newline()),
            ),
            opt(line_ending),
        ),
    )
    .parse_next(input)
    .map(|res| Header {
        company_id: res.0,
        first_valid_date: res.2,
        last_valid_date: res.4,
        version: res.6,
        description: res.8.to_owned(),
    })
}

fn parse_date(input: &mut &str) -> PResult<NaiveDate> {
    take(DATE_FORMAT_LEN)
        .try_map(|s| NaiveDate::parse_from_str(s, DATE_FORMAT))
        .parse_next(input)
}

#[cfg(test)]
mod test_parse_date {
    use super::*;

    #[test]
    fn date_parsing() {
        let input = "01022023";
        let expected = NaiveDate::from_ymd_opt(2023, 2, 1).expect("Valid date");

        assert_eq!(super::parse_date.parse(input).unwrap(), expected);
    }
}

fn parse_time(input: &mut &str) -> PResult<DayOffset> {
    trace("time", take(4u8).try_map(|s: &str| s.parse())).parse_next(input)
}

#[cfg(test)]
mod header_tests {
    use super::*;

    #[test]
    fn test_header_parse() {
        let input = "@100,03072023,04082024,0052,Content description";

        let output = parse_header.parse(input).unwrap();

        assert_eq!(
            output,
            Header {
                company_id: 100,
                version: 52,
                description: "Content description".to_owned(),
                first_valid_date: NaiveDate::from_ymd_opt(2023, 7, 3).unwrap(),
                last_valid_date: NaiveDate::from_ymd_opt(2024, 8, 4).unwrap()
            }
        );
    }
}

fn leg_for_stop(entry: &TimetableEntry) -> Leg {
    let (arrival, departure) = match entry.stop_kind {
        StopKind::Departure(_, scheduled_departure) => {
            (scheduled_departure.offset_by(-1), scheduled_departure)
        }
        StopKind::Arrival(_, scheduled_arrival) => {
            (scheduled_arrival, scheduled_arrival.offset_by(1))
        }
        StopKind::Waypoint => {
            panic!("Shouldn't happen, waypoint should've been filtered out before")
        }
        StopKind::StopShort(_, arrival_departure) => {
            (arrival_departure, arrival_departure.offset_by(1))
        }
        StopKind::StopLong(_, arrival, departure) => (arrival, departure),
    };

    Leg {
        start: arrival,
        end: departure,
        kind: LegKind::Stationary(entry.code.clone(), entry.stop_kind.clone()),
    }
}

pub fn generate_legs(entries: &[TimetableEntry]) -> Vec<Leg> {
    let mut out = vec![];
    let mut waypoints = vec![];
    let first_stop = entries.first().expect("timetable to have an entry");
    let mut previous_stop = first_stop;

    out.push(leg_for_stop(first_stop));

    entries.iter().skip(1).for_each(|entry| {
        // Collect non-stopping points into waypoints. These are needed later on to find the right Links between Stations
        if entry.stop_kind.is_waypoint() {
            waypoints.push(entry);
            return;
        }

        out.push(Leg {
            start: *previous_stop
                .stop_kind
                .departure_time()
                .expect("leg start to have a departure time"),
            end: *entry
                .stop_kind
                .arrival_time()
                .expect("leg end to have an arrival time"),
            kind: LegKind::Moving {
                from: previous_stop.code.clone(),
                to: entry.code.clone(),
                waypoints: waypoints.iter().map(|c| c.code.clone()).collect(),
            },
        });

        previous_stop = entry;

        waypoints.clear();

        out.push(leg_for_stop(entry));
    });

    out
}

// Gets the index of the nth stop, skipping waypoints
fn timetable_stop_index(entries: &[TimetableEntry], nth: usize) -> Option<usize> {
    entries
        .iter()
        .enumerate()
        .filter(|(_, stop)| !stop.stop_kind.is_waypoint())
        .nth(nth)
        .map(|(index, _)| index)
}

fn timetable_start(entries: &[TimetableEntry]) -> DayOffset {
    *entries
        .first()
        .expect("timetable to have an entry")
        .stop_kind
        .departure_time()
        .expect("first entry to have a departure time")
}

fn timetable_end(entries: &[TimetableEntry]) -> DayOffset {
    *entries
        .last()
        .expect("timetable to have an entry")
        .stop_kind
        .arrival_time()
        .expect("last entry to have an arrival time")
}

fn timetable_normalize_ends(entries: &mut [TimetableEntry]) {
    assert!(entries.len() >= 2);

    // Change first entry into a departure
    let departure_time = entries
        .first()
        .unwrap()
        .stop_kind
        .departure_time()
        .expect("stop have departure time");
    let departure_platform = entries.first().unwrap().stop_kind.platform_info().cloned();

    entries.first_mut().unwrap().stop_kind =
        StopKind::Departure(departure_platform, *departure_time);

    // Change last entry into a arrival
    let arrival_time = entries
        .last()
        .unwrap()
        .stop_kind
        .arrival_time()
        .expect("stop to have arrival time");
    let arrival_platform = entries.last().unwrap().stop_kind.platform_info().cloned();

    entries.last_mut().unwrap().stop_kind = StopKind::Arrival(arrival_platform, *arrival_time);
}

impl Ride {
    pub fn start_time(&self) -> DayOffset {
        timetable_start(self.timetable.as_slice())
    }

    pub fn end_time(&self) -> DayOffset {
        timetable_end(self.timetable.as_slice())
    }

    pub fn generate_legs(&self) -> Vec<Leg> {
        generate_legs(&self.timetable)
    }
}

impl Record {
    pub fn start_time(&self) -> DayOffset {
        timetable_start(self.timetable.as_slice())
    }

    pub fn end_time(&self) -> DayOffset {
        timetable_end(self.timetable.as_slice())
    }

    pub fn split_on_ride_id(&self) -> Vec<Ride> {
        let is_sole_transit_type = self.transit_types.len() == 1;

        if !is_sole_transit_type && self.ride_id.len() > 1 {
            println!("{:#?}", self);
            todo!("Support mixed transit_types and rideids")
        }

        self.ride_id
            .iter()
            .enumerate()
            .map(|(index, ride_id)| {
                let transit_type = if is_sole_transit_type {
                    self.transit_types.first().unwrap()
                } else {
                    self.transit_types.get(index).unwrap()
                };

                let first_stop_idx =
                    timetable_stop_index(&self.timetable, ride_id.first_stop as usize - 1)
                        .expect("to find first stop");
                let last_stop_idx =
                    timetable_stop_index(&self.timetable, ride_id.last_stop as usize - 1)
                        .expect("to find last stop");

                let mut timetable = self.timetable[first_stop_idx..=last_stop_idx].to_owned();

                timetable_normalize_ends(&mut timetable);

                let next = self.ride_id.get(index + 1).map(|id| id.ride_id.to_string());

                let is_first = index == 0;
                let previous = if is_first {
                    None
                } else {
                    self.ride_id.get(index - 1).map(|id| id.ride_id.to_string())
                };

                Ride {
                    transit_mode: transit_type.mode.clone(),
                    timetable,
                    id: ride_id.ride_id.to_string(),
                    day_validity: self.day_validity_footnote,
                    next,
                    previous,
                }
            })
            .collect()
    }

    pub(crate) fn generate_legs(&self) -> Vec<Leg> {
        generate_legs(&self.timetable)
    }
}

#[derive(PartialEq, Debug, Serialize, Eq, Clone)]
pub struct TransitMode {
    mode: String,
    first_stop: u32,
    last_stop: u32,
}

fn till_comma<'a>(input: &mut &'a str) -> PResult<&'a str> {
    take_till(0.., |c| c == ',').parse_next(input)
}

fn untill_newline<'a>(input: &mut &'a str) -> PResult<&'a str> {
    take_until(0.., IFF_NEWLINE).parse_next(input)
}

//&IC ,001,005
fn parse_transit_mode(input: &mut &str) -> PResult<TransitMode> {
    (
        "&",
        till_comma,
        ",",
        dec_uint_leading,
        ",",
        dec_uint_leading,
        IFF_NEWLINE,
    )
        .parse_next(input)
        .map(|seq| TransitMode {
            mode: seq.1.trim().to_owned(),
            first_stop: seq.3,
            last_stop: seq.5,
        })
}

fn empty_str_to_none(a: &str) -> Option<&str> {
    if a.is_empty() {
        None
    } else {
        Some(a)
    }
}
