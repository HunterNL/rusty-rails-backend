use std::{collections::HashMap, str::FromStr};

use winnow::{
    ascii::{alphanumeric1, line_ending, multispace0, space0},
    combinator::{dispatch, fail, opt, preceded, repeat, terminated, trace},
    stream::AsChar,
    token::{one_of, take_till, take_while},
    PResult, Parser,
};

use crate::iff::{
    DayValidityFootnote, Footnote, Header, Platform, PlatformInfo, Record, RideId, RideValidity,
    StopKind, TimeTable, TimetableEntry,
};

use super::{
    dec_uint_leading, empty_str_to_none, parse_header, parse_time, parse_transit_mode, seperator,
    untill_newline, TransitMode, IFF_NEWLINE,
};

fn parse_single_day(input: &mut &str) -> PResult<bool> {
    one_of(['0', '1']).map(|char| char == '1').parse_next(input)
}

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
    (parse_header, parse_records)
        .parse_next(input)
        .map(|seq| TimeTable {
            header: seq.0,
            rides: seq.1,
        })
}

fn parse_records(input: &mut &str) -> PResult<Vec<Record>> {
    let estimate_count = input.matches('#').count();
    let mut accumulator = Vec::with_capacity(estimate_count);

    while !input.is_empty() {
        accumulator.push(parse_record.parse_next(input)?)
    }

    Ok(accumulator)
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
        arrival_platform: seq.1.map(|s| Platform::from_str(s).unwrap()),
        departure_platform: seq.4.map(|s| Platform::from_str(s).unwrap()),
        footnote: seq.7,
    })
}

fn parse_departure(input: &mut &str) -> PResult<TimetableEntry> {
    (
        parse_code,
        space0,
        ',',
        parse_time,
        line_ending,
        opt(parse_platform_info),
    )
        .parse_next(input)
        .map(|seq| TimetableEntry {
            code: seq.0,
            stop_kind: StopKind::Departure(seq.5, seq.3),
        })
}

fn any_entry(input: &mut &str) -> PResult<TimetableEntry> {
    dispatch! {winnow::token::any;
            '>' => parse_departure,
            ';' => parse_waypoint,
            '.' => parse_stop_short,
            '+' => parse_stop_long,
            '<' => parse_arrival,
            _ => fail,
    }
    .parse_next(input)
}

fn parse_waypoint(input: &mut &str) -> PResult<TimetableEntry> {
    (parse_code, opt(line_ending))
        .parse_next(input)
        .map(|seq| TimetableEntry {
            code: seq.0,
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
        parse_code,
        ',',
        parse_time,
        line_ending,
        opt(parse_platform_info),
    )
        .parse_next(input)
        .map(|seq| TimetableEntry {
            code: seq.0,
            stop_kind: StopKind::StopShort(seq.4, seq.2),
        })
}

fn parse_stop_long(input: &mut &str) -> PResult<TimetableEntry> {
    (
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
            code: seq.0,
            stop_kind: StopKind::StopLong(seq.6, seq.2, seq.4),
        })
}

fn parse_arrival(input: &mut &str) -> PResult<TimetableEntry> {
    (
        parse_code,
        ',',
        parse_time,
        opt(line_ending),
        opt(parse_platform_info),
    )
        .parse_next(input)
        .map(|seq| TimetableEntry {
            code: seq.0,
            stop_kind: StopKind::Arrival(seq.4, seq.2),
        })
}

#[cfg(test)]
mod test_platform_parse {
    use super::*;

    #[test]
    fn test_platform_parse() {
        let input = "?11 ,15 ,00003";
        let expected = PlatformInfo {
            arrival_platform: Some(Platform::from_str("11").unwrap()),
            departure_platform: Some(Platform::from_str("15").unwrap()),
            footnote: 3,
        };

        assert_eq!(super::parse_platform_info.parse(input).unwrap(), expected);
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
            repeat(1.., any_entry),
        ),
    )
    .map(
        |seq: (_, _, _, _, _, TransitMode, _, Vec<TimetableEntry>)| {
            Record {
                id: seq.0,
                timetable: seq.7,
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
            parsing::{dec_uint_leading, TransitMode},
            Platform, PlatformInfo, Ride, RideId, StopKind, TimetableEntry,
        },
    };

    macro_rules! platform {
        ($platform:literal,$footnote:literal) => {
            PlatformInfo {
                arrival_platform: Some(Platform::plain($platform)),
                departure_platform: Some(Platform::plain($platform)),
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
            .parse(include_str!("../testdata/record1"))
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
                            Some(platform!(13, 3)),
                            DayOffset::from_hour_minute(18, 50)
                        )
                    ),
                    entry!("rtn", StopKind::Waypoint),
                    TimetableEntry {
                        code: "rta".to_owned(),
                        stop_kind: StopKind::StopShort(
                            Some(PlatformInfo {
                                arrival_platform: Some(Platform::plain(1)),
                                departure_platform: Some(Platform::plain(1)),
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
                            Some(platform!(3, 3)),
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
                            Some(platform!(11, 3)),
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
                            Some(platform!(11, 3)),
                            DayOffset::from_hour_minute(19, 36)
                        )
                    ),
                    entry!("uto", StopKind::Waypoint),
                    entry!("bhv", StopKind::Waypoint),
                    entry!("dld", StopKind::Waypoint),
                    entry!(
                        "amf",
                        StopKind::Arrival(
                            Some(platform!(2, 3)),
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
        let record = parse_record.parse(include_str!("../testdata/record1"))?;

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
                        departure_platform: Some(Platform::plain(13)),
                        arrival_platform: Some(Platform::plain(13)),
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
        let input = include_str!("../testdata/record2");

        parse_record.parse(input).map_err(|e| e.to_string())?;

        Ok(())
    }

    #[test]
    fn test_parse_3() -> Result<(), String> {
        let input = include_str!("../testdata/record3");

        parse_record.parse(input).map_err(|e| e.to_string())?;

        Ok(())
    }

    // Tests for record that lack any feature footnotes
    #[test]
    fn test_parse_4() -> TestResult {
        let input = include_str!("../testdata/record4");

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
