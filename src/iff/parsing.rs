use chrono::NaiveDate;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Display;

use winnow::ascii::{alphanumeric1, dec_uint, line_ending, multispace0, space0, Uint};
use winnow::combinator::trace;
use winnow::combinator::{alt, delimited, fail, opt, preceded, repeat, terminated};
use winnow::stream::AsChar;

use winnow::token::{one_of, take, take_till, take_until, take_while};
use winnow::{PResult, Parser};

use crate::dayoffset::DayOffset;

use super::{
    DayValidityFootnote, Footnote, Header, Leg, LegKind, PlatformInfo, Record, Ride, RideId,
    RideValidity, StopKind, TimeTable, TimetableEntry,
};

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

fn parse_single_day(input: &mut &str) -> PResult<bool> {
    one_of(['0', '1']).map(|char| char == '1').parse_next(input)
}

// fn take_zeroes<'a>(input: &mut &'a str) -> PResult<&'a str> {
//     take_while(0.., '0').parse_next(input)
// }

// fn parse_uint_leading<'s, U: Uint, A>(input: &mut &'s str) -> PResult<&'s str> {
//     digit1.parse_next(input)
// }

// fn skip_zeroes(input: &mut &str) -> PResult<&str> {

// // }

fn dec_uint_leading<Output: Uint + Clone>(input: &mut &str) -> PResult<Output> {
    alt((
        (take_while(0.., '0'), dec_uint).map(|a| a.1),
        (take_while(1.., '0')).value(Output::try_from_dec_uint("0").unwrap()),
        // (take_while(1.., '0')).value(0),
        fail,
    ))
    .parse_next(input)
}

// fn get_uint_as_trait<U: Uint>() -> U {
//     let var_name = 0 as U;
//     var_name
// }

// fn foo() {
//     let a: u8 = get_uint_as_trait();
//     let b = a;
// }
// TODO, pass 0 as proper type right away?

// let zeroes = take_zeroes.parse_next(input)?;
// let has_zeroes = zeroes.len() > 0;

// let b = match dec_uint.parse_next(input) {
//     Ok(a) => Ok(a),
//     Err(a) => todo!(),
// }

// let digits = digit0.parse_next(input)?;

// PResult::ok(&mut input)

fn parse_footnote_record(input: &mut &str) -> PResult<DayValidityFootnote> {
    (
        '#',
        dec_uint_leading,
        line_ending,
        repeat(1.., parse_single_day),
        line_ending,
    )
        .map(|seq| DayValidityFootnote {
            id: seq.1,
            validity: seq.3,
        })
        .parse_next(input)
}

pub fn parse_footnote_file(input: &mut &str) -> PResult<RideValidity> {
    (parse_header, repeat(0.., parse_footnote_record))
        .map(|seq: (Header, Vec<DayValidityFootnote>)| RideValidity {
            header: seq.0,
            validities: seq
                .1
                .into_iter()
                .map(|footnote| (footnote.id, footnote.validity))
                .collect::<HashMap<_, _>>(),
        })
        .parse_next(input)
}

pub fn parse_timetable_file(input: &mut &str) -> PResult<TimeTable> {
    (parse_header, repeat(0.., parse_record))
        .parse_next(input)
        .map(|seq| TimeTable {
            header: seq.0,
            rides: seq.1,
        })
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

// ?13 ,13 ,00003
// ?1-2  ,1-2  ,00081
// ?     ,     ,00187
fn parse_platform_info(input: &mut &str) -> PResult<PlatformInfo> {
    trace(
        "platform_info",
        (
            '?',
            opt(take_while(1.., ('-', AsChar::is_alphanum))),
            multispace0,
            ',',
            opt(take_while(1.., ('-', AsChar::is_alphanum))),
            multispace0,
            seperator,
            dec_uint_leading::<u64>,
            opt(line_ending), // (take_while(1.., |c| !AsChar::is_newline(c)),),
        ),
    )
    .parse_next(input)
    .map(|seq| PlatformInfo {
        arrival_platform: seq.1.map(std::borrow::ToOwned::to_owned),
        departure_platform: seq.4.map(std::borrow::ToOwned::to_owned),
        footnote: seq.7,
    })
}

#[cfg(test)]
mod test_platform_parse {
    use super::*;

    #[test]
    fn test_platform_parse() {
        let input = "?11 ,15 ,00003";
        let expected = PlatformInfo {
            arrival_platform: Some("11".to_owned()),
            departure_platform: Some("15".to_owned()),
            footnote: 3,
        };

        assert_eq!(super::parse_platform_info.parse(input).unwrap(), expected);
    }
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
            kind: LegKind::Moving(
                previous_stop.code.clone(),
                entry.code.clone(),
                waypoints.iter().map(|c| c.code.clone()).collect(),
            ),
        });

        previous_stop = entry;

        waypoints.clear();

        out.push(leg_for_stop(entry));
    });

    out
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

fn parse_departure(input: &mut &str) -> PResult<TimetableEntry> {
    preceded(
        '>',
        (
            parse_code,
            space0,
            ',',
            parse_time,
            line_ending,
            opt(parse_platform_info),
        ),
    )
    .parse_next(input)
    .map(|seq| TimetableEntry {
        code: seq.0,
        stop_kind: StopKind::Departure(seq.5, seq.3),
    })
}

fn any_entry(input: &mut &str) -> PResult<TimetableEntry> {
    trace(
        "any_entry",
        alt((
            parse_departure,
            parse_waypoint,
            parse_stop_short,
            parse_stop_long,
            parse_arrival,
            fail,
        )),
    )
    .parse_next(input)
}

fn parse_waypoint(input: &mut &str) -> PResult<TimetableEntry> {
    (';', parse_code, opt(line_ending))
        .parse_next(input)
        .map(|seq| TimetableEntry {
            code: seq.1,
            stop_kind: StopKind::Waypoint,
        })
}

fn parse_code(input: &mut &str) -> PResult<String> {
    terminated(alphanumeric1, multispace0)
        .parse_next(input)
        .map(std::borrow::ToOwned::to_owned)
}

fn parse_stop_short(input: &mut &str) -> PResult<TimetableEntry> {
    (
        '.',
        parse_code,
        ',',
        parse_time,
        line_ending,
        opt(parse_platform_info),
    )
        .parse_next(input)
        .map(|seq| TimetableEntry {
            code: seq.1,
            stop_kind: StopKind::StopShort(seq.5, seq.3),
        })
}

fn parse_stop_long(input: &mut &str) -> PResult<TimetableEntry> {
    (
        '+',
        parse_code,
        ',',
        parse_time,
        ',',
        parse_time,
        line_ending,
        opt(parse_platform_info),
    )
        .parse_next(input)
        .map(|seq| TimetableEntry {
            code: seq.1,
            stop_kind: StopKind::StopLong(seq.7, seq.3, seq.5),
        })
}

fn parse_arrival(input: &mut &str) -> PResult<TimetableEntry> {
    (
        '<',
        parse_code,
        ',',
        parse_time,
        opt(line_ending),
        opt(parse_platform_info),
    )
        .parse_next(input)
        .map(|seq| TimetableEntry {
            code: seq.1,
            stop_kind: StopKind::Arrival(seq.5, seq.3),
        })
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

//%100,02871, ,001,004,
fn parse_ride_id(input: &mut &str) -> PResult<RideId> {
    (
        '%',
        dec_uint_leading,
        ',',
        dec_uint_leading,
        ',',
        opt(dec_uint_leading),
        space0,
        ',',
        dec_uint_leading,
        ',',
        dec_uint_leading,
        ',',
        untill_newline,
        IFF_NEWLINE,
    )
        .map(|seq| RideId {
            company_id: seq.1,
            ride_id: seq.3,
            line_id: seq.5,
            first_stop: seq.8,
            last_stop: seq.10,
            ride_name: empty_str_to_none(str::trim(seq.12)).map(std::borrow::ToOwned::to_owned),
        })
        .parse_next(input)
}

#[cfg(test)]
mod test_rideid_parse {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn plain() {
        let input = "%100,02871, ,001,004,                               \r\n";
        let expected = RideId {
            company_id: 100,
            first_stop: 1,
            last_stop: 4,
            ride_id: 2871,
            line_id: None,
            ride_name: None,
        };

        assert_eq!(parse_ride_id.parse(input).unwrap(), expected);
    }
}

fn parse_day_footnote(input: &mut &str) -> PResult<Footnote> {
    preceded(
        '-',
        (
            dec_uint_leading,
            ',',
            dec_uint_leading,
            ',',
            dec_uint_leading,
            line_ending,
        ),
    )
    .map(|seq| Footnote {
        footnote: seq.0,
        first_stop: seq.2,
        last_stop: seq.4,
    })
    .parse_next(input)
}

fn parse_record(input: &mut &str) -> PResult<Record> {
    preceded(
        '#',
        (
            dec_uint_leading,
            line_ending,
            repeat(0.., parse_ride_id),
            parse_day_footnote,
            take_till(0.., '&').void(),
            parse_transit_mode,
            take_till(0.., '>').void(),
            parse_departure,
            repeat(1.., any_entry),
        ),
    )
    .map(
        |seq: (_, _, _, _, _, TransitMode, _, _, Vec<TimetableEntry>)| {
            let mut v = vec![seq.7];
            v.extend(seq.8);
            Record {
                id: seq.0,
                timetable: v,
                ride_id: seq.2,
                day_validity_footnote: seq.3.footnote, // NONSTANDARD assuming date footnotes span the entire length of a record
                transit_types: vec![seq.5],
            }
        },
    )
    .parse_next(input)
}

#[cfg(test)]
mod test_record {
    use pretty_assertions::assert_eq;

    use testresult::TestResult;
    use winnow::Parser;

    use crate::{
        dayoffset::DayOffset,
        iff::{
            parsing::{dec_uint_leading, RideId, TimetableEntry, TransitMode},
            PlatformInfo, Ride, StopKind,
        },
    };

    macro_rules! platform {
        ($platform:literal,$footnote:literal) => {
            PlatformInfo {
                arrival_platform: Some($platform.to_owned()),
                departure_platform: Some($platform.to_owned()),
                footnote: $footnote,
            }
        };
    }

    macro_rules! entry {
        ($station:literal,$stop:expr) => {
            TimetableEntry {
                code: $station.to_owned(),
                stop_kind: $stop,
            }
        };
    }

    use super::parse_record;

    #[test]
    fn test_record_split() -> TestResult {
        let record = parse_record
            .parse(include_str!("./testdata/record1"))
            .unwrap();

        let rides = record.split_on_ride_id();

        assert_eq!(rides.len(), 2);
        #[allow(clippy::get_first)]
        let ride0 = rides.get(0).unwrap();
        let ride1 = rides.get(1).unwrap();

        assert_eq!(
            ride0,
            &Ride {
                id: "2871".to_owned(),
                transit_mode: "IC".to_owned(),
                timetable: vec![
                    entry!(
                        "rtd",
                        StopKind::Departure(
                            Some(platform!("13", 3)),
                            DayOffset::from_hour_minute(18, 50)
                        )
                    ),
                    entry!("rtn", StopKind::Waypoint),
                    TimetableEntry {
                        code: "rta".to_owned(),
                        stop_kind: StopKind::StopShort(
                            Some(PlatformInfo {
                                arrival_platform: Some("1".to_owned()),
                                departure_platform: Some("1".to_owned()),
                                footnote: 3
                            }),
                            DayOffset::from_hour_minute(18, 58)
                        )
                    },
                    entry!("cps", StopKind::Waypoint),
                    entry!("nwk", StopKind::Waypoint),
                    entry!(
                        "gd",
                        StopKind::StopLong(
                            Some(platform!("3", 3)),
                            DayOffset::from_hour_minute(19, 8),
                            DayOffset::from_hour_minute(19, 9)
                        )
                    ),
                    entry!("gdg", StopKind::Waypoint),
                    entry!("wd", StopKind::Waypoint),
                    entry!("vtn", StopKind::Waypoint),
                    entry!("utt", StopKind::Waypoint),
                    entry!("utlr", StopKind::Waypoint),
                    entry!(
                        "ut",
                        StopKind::Arrival(
                            Some(platform!("11", 3)),
                            DayOffset::from_hour_minute(19, 28)
                        )
                    ),
                ],
                day_validity: 3,
                previous: None,
                next: Some("1771".to_owned())
            }
        );

        assert_eq!(
            ride1,
            &Ride {
                id: "1771".to_owned(),
                transit_mode: "IC".to_owned(),
                timetable: vec![
                    entry!(
                        "ut",
                        StopKind::Departure(
                            Some(platform!("11", 3)),
                            DayOffset::from_hour_minute(19, 36)
                        )
                    ),
                    entry!("uto", StopKind::Waypoint),
                    entry!("bhv", StopKind::Waypoint),
                    entry!("dld", StopKind::Waypoint),
                    entry!(
                        "amf",
                        StopKind::Arrival(
                            Some(platform!("2", 3)),
                            DayOffset::from_hour_minute(19, 50)
                        )
                    ),
                ],
                day_validity: 3,
                previous: Some("2871".to_owned()),
                next: None
            }
        );

        Ok(())
    }

    #[test]
    fn test_record_parse() -> TestResult {
        let record = parse_record.parse(include_str!("./testdata/record1"))?;

        assert_eq!(record.id, 2);

        assert_eq!(
            record.transit_types.first().unwrap(),
            &TransitMode {
                mode: "IC".to_owned(),
                first_stop: 1,
                last_stop: 5
            }
        );

        assert_eq!(record.day_validity_footnote, 3);

        assert_eq!(
            record.ride_id,
            vec![
                RideId {
                    company_id: 100,
                    first_stop: 1,
                    last_stop: 4,
                    ride_id: 2871,
                    line_id: None,
                    ride_name: None
                },
                RideId {
                    company_id: 100,
                    first_stop: 4,
                    last_stop: 5,
                    ride_id: 1771,
                    line_id: None,
                    ride_name: None
                }
            ]
        );

        assert_eq!(
            record.timetable.first().unwrap(),
            &TimetableEntry {
                code: "rtd".to_owned(),
                stop_kind: StopKind::Departure(
                    Some(PlatformInfo {
                        departure_platform: Some("13".to_owned()),
                        arrival_platform: Some("13".to_owned()),
                        footnote: 3
                    }),
                    DayOffset::from_hour_minute(18, 50)
                )
            }
        );
        Ok(())
    }

    #[test]
    fn record_parse_2() -> Result<(), String> {
        let input = include_str!("./testdata/record2");

        parse_record.parse(input).map_err(|e| e.to_string())?;

        Ok(())
    }

    #[test]
    fn test_parse_3() -> Result<(), String> {
        let input = include_str!("./testdata/record3");

        parse_record.parse(input).map_err(|e| e.to_string())?;

        Ok(())
    }

    // Tests for record that lack any feature footnotes
    #[test]
    fn test_parse_4() -> TestResult {
        let input = include_str!("./testdata/record4");

        parse_record.parse(input).map_err(|e| e.to_string())?;

        Ok(())
    }

    #[test]
    fn uint_content() {
        let out: u32 = (dec_uint_leading)
            .parse("123")
            .map_err(|e| e.to_string())
            .unwrap();

        assert_eq!(out, 123);
    }

    #[test]
    fn uint_leading_content() {
        let out: u32 = (dec_uint_leading)
            .parse("000123")
            .map_err(|e| e.to_string())
            .unwrap();

        assert_eq!(out, 123);
    }

    #[test]
    fn uint_leading_empty() {
        let out: u32 = (dec_uint_leading)
            .parse("000000")
            .map_err(|e| e.to_string())
            .unwrap();

        assert_eq!(out, 0);
    }

    #[test]
    fn uint_leading_none() {
        let out: Result<u32, String> = (dec_uint_leading).parse("").map_err(|e| e.to_string());

        assert!(out.is_err())
    }
}
