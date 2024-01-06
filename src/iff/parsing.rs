use chrono::{NaiveDate, NaiveTime};
use std::fmt::Display;
use std::{fs::File, io::Read};
use winnow::ascii::{
    alphanumeric0, alphanumeric1, dec_uint, line_ending, multispace0, multispace1, space0, space1,
};
use winnow::combinator::{alt, delimited, eof, fail, not, opt, preceded, repeat, terminated};
use winnow::stream::AsChar;
use winnow::trace::trace;

use winnow::token::{one_of, take, take_till, take_while};
use winnow::{PResult, Parser};

use super::dayoffset::DayOffset;

const TIMETABLE_FILE_NAME: &str = "timetbls.dat";
const DATE_FORMAT_LEN: usize = "DDMMYYYY".len(); // Lenght of dates as they appear in the iff file
const DATE_FORMAT: &str = "%d%m%Y";

pub struct Iff {
    pub header: Header,
    pub rides: Vec<Record>,
}

pub struct InvalidEncodingError {}

impl Display for InvalidEncodingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("File was not encoded as valid UTF-8")
    }
}

fn string1<'a>(input: &mut &'a str) -> PResult<&'a str> {
    take_while(1.., |c: char| c.is_ascii_alphanumeric() || c.is_space()).parse_next(input)
}

fn spacecomma<'a>(input: &mut &'a str) -> PResult<&'a str> {
    " ,".parse_next(input)
}

fn seperator<'a>(input: &'a mut &str) -> PResult<()> {
    (multispace0, ',').void().parse_next(input)
}

impl Iff {
    pub fn from_file(file: &File) -> Result<Self, String> {
        let mut archive = zip::ZipArchive::new(file).expect("valid new archive");
        let mut timetable_file = archive
            .by_name(TIMETABLE_FILE_NAME)
            .map_err(|_| "Error getting file from archive")?;

        let mut buf = vec![];

        timetable_file
            .read_to_end(&mut buf)
            .map_err(|e| e.to_string())?;

        let str_content = std::str::from_utf8(buf.as_slice())
            .map_err(|_| "Timetable file contained invalid utf-8")?;

        parse_IFF.parse(str_content).map_err(|o| o.to_string())
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

fn parse_IFF<'a>(input: &'a mut &str) -> PResult<Iff> {
    (parse_header, repeat(0.., parse_record))
        .parse_next(input)
        .map(|seq| Iff {
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
                dec_uint,
                ',',
                parse_date,
                ',',
                parse_date,
                ',',
                dec_uint,
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

#[derive(Debug, PartialEq, Eq)]
pub struct PlatformInfo {
    arrival_platform: Option<String>,
    departure_platform: Option<String>,
    footnote: u64,
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
            dec_uint::<_, u64, _>,
            opt(line_ending), // (take_while(1.., |c| !AsChar::is_newline(c)),),
        ),
    )
    .parse_next(input)
    .map(|seq| PlatformInfo {
        arrival_platform: seq.1.map(|s| s.to_owned()),
        departure_platform: seq.4.map(|s| s.to_owned()),
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

fn parse_date<'a>(input: &'a mut &str) -> PResult<NaiveDate> {
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

        assert_eq!(super::parse_date.parse(input).unwrap(), expected)
    }
}

fn parse_time<'a>(input: &'a mut &str) -> PResult<DayOffset> {
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
        )
    }
}

#[derive(PartialEq, Debug, Eq)]
pub struct TimetableEntry {
    pub code: String,
    pub stop_kind: StopKind,
}

#[derive(PartialEq, Debug, Eq)]
pub enum StopKind {
    Departure(Option<PlatformInfo>, DayOffset),
    Arrival(Option<PlatformInfo>, DayOffset),
    Waypoint,
    StopShort(Option<PlatformInfo>, DayOffset),
    StopLong(Option<PlatformInfo>, DayOffset, DayOffset),
}

#[derive(PartialEq, Debug, Eq)]
pub struct Record {
    pub id: u64,
    pub timetable: Vec<TimetableEntry>,
}

fn parse_departure<'a>(input: &'a mut &str) -> PResult<TimetableEntry> {
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

fn any_entry<'a>(input: &'a mut &str) -> PResult<TimetableEntry> {
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
        .map(|str| str.to_owned())
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

fn parse_record<'a>(input: &'a mut &str) -> PResult<Record> {
    preceded(
        '#',
        (
            dec_uint,
            take_till(1.., '>').void(),
            parse_departure,
            repeat(1.., any_entry),
        ),
    )
    .parse_next(input)
    .map(|seq: (_, _, _, Vec<TimetableEntry>)| {
        let mut v = vec![seq.2];
        v.extend(seq.3);
        Record {
            id: seq.0,
            timetable: v,
        }
    })
}

#[cfg(test)]
mod test_record {

    use std::error;

    use winnow::Parser;

    use crate::iff::{
        dayoffset::{self, DayOffset},
        parsing::{PlatformInfo, StopKind, TimetableEntry},
    };

    use super::parse_record;

    #[test]
    fn test_record_parse() {
        let mut input = "#00000002
%100,02871, ,001,004,
%100,01771, ,004,005,
-00003,000,999
&IC ,001,005
*FINI,001,004,00000
*FINI,004,005,00000
>rtd ,1850
?13 ,13 ,00003
;rtn
.rta ,1858
?1 ,1 ,00003
;cps
;nwk
+gd ,1908,1909
?3 ,3 ,00003
;gdg
;wd
;vtn
;utt
;utlr
+ut ,1928,1936a
?11 ,11 ,00003
;uto
;bhv
;dld
<amf ,1950
?2 ,2 ,00003";

        let record = super::parse_record.parse_next(&mut input).unwrap();

        assert_eq!(record.id, 2);

        assert_eq!(
            record.timetable.get(0).unwrap(),
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
        )
    }

    #[test]
    fn record_parse_2() -> Result<(), String> {
        let input = "#00001283
%200,09316,      ,001,005,                              
%200,09916,      ,005,007,                              
-00081,000,999
&EST ,001,007
*BAR ,001,005,00000
*FINI,001,005,00000
*RESV,001,005,00000
*ROL ,001,005,00000
*SPEC,001,005,00000
*BAR ,005,007,00000
*FINI,005,007,00000
*RESV,005,007,00000
*ROL ,005,007,00000
*SPEC,005,007,00000
*NUIT,002,003,00000
>asd    ,0715
?14   ,14   ,00081
;ass    
;asdl   
+shl    ,0730,0732
?1-2  ,1-2  ,00081
;hfd    
+rtd    ,0754,0758
?2    ,2    ,00081
;rtb    
;rtz    
;rtst   
;rlb    
;ndkp   
;atwlb  
+atw    ,0830,0833
;berch  
;gmd    
;gmog   
;mho    
;fki    
;fdp    
;fwa    
;lnk    
;mech   
;fbnl   
;brusn  
;brusc  
+brusz  ,0908,0920
+acdg   ,1033,1038
?1    ,1    ,00081
<marne  ,1048";

        parse_record.parse(input).map_err(|e| e.to_string())?;

        Ok(())
    }

    #[test]
    fn test_parse_3() -> Result<(), String> {
        let input = "#00002871
%200,00140,      ,001,014,                              
-00187,000,999
&IC  ,001,014
*RESA,001,014,00459
*RESV,001,014,00460
*FIVE,001,014,00000
*NUIT,002,003,00460
*NIIN,012,013,00000
>bhf    ,1554
+berhbl ,1603,1607
+bspd   ,1624,1626
;lrw    
;ls     
;hwob   
+hann   ,1753,1756
;minden 
;oeynh  
+buende ,1841,1843
?     ,     ,00187
+osnh   ,1904,1906
+rheine ,1933,1936
?     ,     ,00187
+bh     ,1948,1951
;odz    
;hglo   
+hgl    ,2007,2009
?2    ,2    ,00187
;bn     
;amri   
;aml    
;wdn    
;rsn    
;hon    
;dvc    
+dv     ,2041,2045
?3    ,3    ,00187
;twl    
;apdo   
+apd    ,2057,2059
?1    ,1    ,00187
;hvl    
+amf    ,2124,2126
?7    ,7    ,00187
;brn    
+hvs    ,2138,2139
?5    ,5    ,00187
;hvsm   
;bsmz   
;ndb    
;wp     
;dmn    
;assp   
;asdm   
<asd    ,2200
?15a  ,15a  ,00187";

        parse_record.parse(input).map_err(|e| e.to_string())?;

        Ok(())
    }
}
