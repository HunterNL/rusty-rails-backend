use std::collections::HashMap;

use winnow::{
    ascii::{alphanumeric1, line_ending, multispace0, space0},
    combinator::{dispatch, fail, opt, preceded, repeat, terminated, trace},
    token::{one_of, take_till},
    PResult, Parser,
};

use crate::iff::{
    DayValidityFootnote, Footnote, Header, LocationCache, Platform, PlatformInfo, Record, RideId,
    RideValidity, StopKind, TimeTable, TimetableEntryRaw,
};

use super::{
    dec_uint_leading, empty_str_to_none, parse_header, parse_time, parse_transit_mode,
    untill_newline, Stream, TransitMode, IFF_NEWLINE,
};

fn parse_single_day(input: &mut Stream) -> PResult<bool> {
    one_of(['0', '1'])
        .map(|char| char == b'1')
        .parse_next(input)
}

struct RecordParser<'a> {
    locations: &'a mut LocationCache,
}

impl<'a, 'b> Parser<Stream<'a>, Record, winnow::error::ContextError> for RecordParser<'b> {
    fn parse_next(&mut self, input: &mut &'a winnow::BStr) -> PResult<Record> {
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
            |seq: (_, _, _, _, _, TransitMode, _, Vec<TimetableEntryRaw>)| {
                Record {
                    id: seq.0,
                    timetable: seq.7.iter().map(|s| s.to_proper(self.locations)).collect(),
                    ride_id: seq.2,
                    day_validity_footnote: seq.3.footnote, // NONSTANDARD assuming date footnotes span the entire length of a record
                    transit_types: vec![seq.5],
                }
            },
        )
        .parse_next(input)
    }
}

fn parse_footnote_record(input: &mut Stream) -> PResult<DayValidityFootnote> {
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

pub fn parse_footnote_file(input: &mut Stream) -> PResult<RideValidity> {
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

pub fn parse_timetable_file(input: &mut Stream<'_>) -> PResult<(TimeTable, LocationCache)> {
    (parse_header, parse_records).parse_next(input).map(|seq| {
        (
            TimeTable {
                header: seq.0,
                rides: seq.1 .0,
            },
            seq.1 .1,
        )
    })
}

fn parse_records(input: &mut Stream) -> PResult<(Vec<Record>, LocationCache)> {
    let estimate_record_count = input.iter().filter(|b| b == &&b'#').count();
    let mut accumulator = Vec::with_capacity(estimate_record_count);

    const ESTIMATE_UNIQUE_LOCATION_CODES: usize = 1000;
    let mut location_codes = LocationCache::with_capacity(ESTIMATE_UNIQUE_LOCATION_CODES);

    let mut record_parser = RecordParser {
        locations: &mut location_codes,
    };

    while !input.is_empty() {
        let record = record_parser.parse_next(input)?;
        accumulator.push(record)
    }

    Ok((accumulator, location_codes))
}

fn parse_platform_opt(input: &mut Stream) -> PResult<Option<Platform>> {
    // trace(
    // "platform",
    take_till(1.., ',')
        .map(|s| unsafe { std::str::from_utf8_unchecked(s) })
        .map(|s| s.parse::<Platform>().ok())
        .parse_next(input)
}

// ?13 ,13 ,00003
// ?1-2  ,1-2  ,00081
// ?     ,     ,00187
fn parse_platform_info(input: &mut Stream) -> PResult<PlatformInfo> {
    trace(
        "platform_info",
        (
            '?',
            parse_platform_opt,
            ',',
            parse_platform_opt,
            ',',
            dec_uint_leading::<u64>,
            opt(line_ending), // (take_while(1.., |c| !AsChar::is_newline(c)),),
        ),
    )
    .parse_next(input)
    .map(|seq| PlatformInfo {
        arrival_platform: seq.1,
        departure_platform: seq.3,
        footnote: seq.5,
    })
}

fn parse_departure<'s>(input: &mut Stream<'s>) -> PResult<TimetableEntryRaw<'s>> {
    (
        parse_code,
        space0,
        ',',
        parse_time,
        line_ending,
        opt(parse_platform_info),
    )
        .parse_next(input)
        .map(|seq| TimetableEntryRaw {
            code: unsafe { std::str::from_utf8_unchecked(seq.0) },
            stop_kind: StopKind::Departure(seq.5, seq.3),
        })
}

fn any_entry<'s>(input: &mut Stream<'s>) -> PResult<TimetableEntryRaw<'s>> {
    dispatch! {winnow::token::any;
            b'>' => parse_departure,
            b';' => parse_waypoint,
            b'.' => parse_stop_short,
            b'+' => parse_stop_long,
            b'<' => parse_arrival,
            _ => fail,
    }
    .parse_next(input)
}

fn parse_waypoint<'s>(input: &mut Stream<'s>) -> PResult<TimetableEntryRaw<'s>> {
    (parse_code, opt(line_ending))
        .parse_next(input)
        .map(|seq| TimetableEntryRaw {
            code: unsafe { std::str::from_utf8_unchecked(seq.0) },
            stop_kind: StopKind::Waypoint,
        })
}

fn parse_code<'s>(input: &mut Stream<'s>) -> PResult<&'s [u8]> {
    terminated(alphanumeric1, multispace0).parse_next(input)
}

fn parse_stop_short<'s>(input: &mut Stream<'s>) -> PResult<TimetableEntryRaw<'s>> {
    (
        parse_code,
        ',',
        parse_time,
        line_ending,
        opt(parse_platform_info),
    )
        .parse_next(input)
        .map(|seq| TimetableEntryRaw {
            code: unsafe { std::str::from_utf8_unchecked(seq.0) },
            stop_kind: StopKind::StopShort(seq.4, seq.2),
        })
}

fn parse_stop_long<'s>(input: &mut Stream<'s>) -> PResult<TimetableEntryRaw<'s>> {
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
        .map(|seq| TimetableEntryRaw {
            code: unsafe { std::str::from_utf8_unchecked(seq.0) },
            stop_kind: StopKind::StopLong(seq.6, seq.2, seq.4),
        })
}

fn parse_arrival<'s>(input: &mut Stream<'s>) -> PResult<TimetableEntryRaw<'s>> {
    (
        parse_code,
        ',',
        parse_time,
        opt(line_ending),
        opt(parse_platform_info),
    )
        .parse_next(input)
        .map(|seq| TimetableEntryRaw {
            code: unsafe { std::str::from_utf8_unchecked(seq.0) },
            stop_kind: StopKind::Arrival(seq.4, seq.2),
        })
}

#[cfg(test)]
mod test_platform_parse {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_platform_parse() {
        let input = "?11 ,15 ,00003".into();
        let expected = PlatformInfo {
            arrival_platform: Some(Platform::plain(11)),
            departure_platform: Some(Platform::plain(15)),
            footnote: 3,
        };

        assert_eq!(super::parse_platform_info.parse(input).unwrap(), expected);
    }
}

//%100,02871, ,001,004,
fn parse_ride_id(input: &mut Stream) -> PResult<RideId> {
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
            ride_name: empty_str_to_none(unsafe { std::str::from_utf8_unchecked(seq.12).trim() })
                .map(std::borrow::ToOwned::to_owned),
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

        assert_eq!(parse_ride_id.parse(input.into()).unwrap(), expected);
    }
}

fn parse_day_footnote(input: &mut Stream) -> PResult<Footnote> {
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

#[cfg(test)]
mod test_record {
    use pretty_assertions::assert_eq;

    use testresult::TestResult;
    use winnow::{BStr, Parser};

    use crate::{
        dayoffset::DayOffset,
        iff::{
            parsing::{dec_uint_leading, timetable::RecordParser, TransitMode},
            LocationCache, Platform, PlatformInfo, Ride, RideId, StopKind, TimetableEntry,
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
        ($handle:expr,$stop:expr) => {
            TimetableEntry {
                code: $handle,
                stop_kind: $stop,
            }
        };
    }

    // use super::parse_record;

    #[test]
    fn test_record_split() -> TestResult {
        let input = include_str!("../testdata/record1");
        let input = BStr::new(input);
        if !input.is_ascii() {
            return Err("Input ins't ASCII".into());
        }
        let mut locations = LocationCache::new();
        let mut record_parser = RecordParser {
            locations: &mut locations,
        };
        let record = record_parser.parse(input).unwrap();

        let rides: Vec<Ride> = record.split_on_ride_id().collect();

        let code = |a: &'static str| locations.lookup_handle(a).unwrap();

        assert_eq!(rides.len(), 2);
        #[allow(clippy::get_first)]
        let ride0 = rides.get(0).unwrap();
        let ride1 = rides.get(1).unwrap();

        assert_eq!(
            ride0,
            &Ride {
                id: "2871".to_owned(),
                operator: 100,
                transit_mode: "IC".to_owned(),
                timetable: vec![
                    entry!(
                        code("rtd"),
                        StopKind::Departure(
                            Some(platform!(13, 3)),
                            DayOffset::from_hour_minute(18, 50)
                        )
                    ),
                    entry!(code("rtn"), StopKind::Waypoint),
                    TimetableEntry {
                        code: code("rta"),
                        stop_kind: StopKind::StopShort(
                            Some(PlatformInfo {
                                arrival_platform: Some(Platform::plain(1)),
                                departure_platform: Some(Platform::plain(1)),
                                footnote: 3
                            }),
                            DayOffset::from_hour_minute(18, 58)
                        )
                    },
                    entry!(code("cps"), StopKind::Waypoint),
                    entry!(code("nwk"), StopKind::Waypoint),
                    entry!(
                        code("gd"),
                        StopKind::StopLong(
                            Some(platform!(3, 3)),
                            DayOffset::from_hour_minute(19, 8),
                            DayOffset::from_hour_minute(19, 9)
                        )
                    ),
                    entry!(code("gdg"), StopKind::Waypoint),
                    entry!(code("wd"), StopKind::Waypoint),
                    entry!(code("vtn"), StopKind::Waypoint),
                    entry!(code("utt"), StopKind::Waypoint),
                    entry!(code("utlr"), StopKind::Waypoint),
                    entry!(
                        code("ut"),
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
                operator: 100,
                id: "1771".to_owned(),
                transit_mode: "IC".to_owned(),
                timetable: vec![
                    entry!(
                        code("ut"),
                        StopKind::Departure(
                            Some(platform!(11, 3)),
                            DayOffset::from_hour_minute(19, 36)
                        )
                    ),
                    entry!(code("uto"), StopKind::Waypoint),
                    entry!(code("bhv"), StopKind::Waypoint),
                    entry!(code("dld"), StopKind::Waypoint),
                    entry!(
                        code("amf"),
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
        let input = include_str!("../testdata/record1");
        let input = BStr::new(input);
        if !input.is_ascii() {
            return Err("Input ins't ASCII".into());
        }
        let mut locations = LocationCache::new();
        let mut record_parser = RecordParser {
            locations: &mut locations,
        };

        let record = record_parser.parse(input)?;
        let code = |a: &'static str| locations.lookup_handle(a).unwrap();

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
                code: code("rtd"),
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
        let input = BStr::new(input);
        if !input.is_ascii() {
            return Err("Input ins't ASCII".into());
        }
        let mut locations = LocationCache::new();
        let mut record_parser = RecordParser {
            locations: &mut locations,
        };

        record_parser.parse(input).map_err(|e| e.to_string())?;

        Ok(())
    }

    #[test]
    fn test_parse_3() -> Result<(), String> {
        let input = include_str!("../testdata/record3");
        let input = BStr::new(input);
        if !input.is_ascii() {
            return Err("Input ins't ASCII".into());
        }
        let mut locations = LocationCache::new();
        let mut record_parser = RecordParser {
            locations: &mut locations,
        };

        record_parser.parse(input).map_err(|e| e.to_string())?;

        Ok(())
    }

    // Tests for record that lack any feature footnotes
    #[test]
    fn test_parse_4() -> TestResult {
        let input = include_str!("../testdata/record4");
        let input = BStr::new(input);
        if !input.is_ascii() {
            return Err("Input ins't ASCII".into());
        }
        let mut locations = LocationCache::new();
        let mut record_parser = RecordParser {
            locations: &mut locations,
        };

        record_parser.parse(input).map_err(|e| e.to_string())?;

        Ok(())
    }

    #[test]
    fn uint_content() {
        let out: u32 = (dec_uint_leading)
            .parse("123".into())
            .map_err(|e| e.to_string())
            .unwrap();

        assert_eq!(out, 123);
    }

    #[test]
    fn uint_leading_content() {
        let out: u32 = (dec_uint_leading)
            .parse("000123".into())
            .map_err(|e| e.to_string())
            .unwrap();

        assert_eq!(out, 123);
    }

    #[test]
    fn uint_leading_empty() {
        let out: u32 = (dec_uint_leading)
            .parse("000000".into())
            .map_err(|e| e.to_string())
            .unwrap();

        assert_eq!(out, 0);
    }

    #[test]
    fn uint_leading_none() {
        let out: Result<u32, String> = (dec_uint_leading)
            .parse("".into())
            .map_err(|e| e.to_string());

        assert!(out.is_err())
    }
}
