use winnow::{combinator::repeat, error::ParseError, PResult, Parser};

use crate::iff::{Company, Header};

use super::{dec_uint_leading, parse_header, parse_time, till_comma, Stream, IFF_NEWLINE};

pub struct CompanyFile {
    pub header: Header,
    pub companies: Vec<Company>,
}

// 100,ns        ,NS                            ,0400
pub fn parse_company(input: &mut Stream<'_>) -> PResult<Company> {
    (
        till_comma.and_then(dec_uint_leading),
        ",",
        till_comma.map(|s| unsafe { std::str::from_utf8_unchecked(s).trim() }),
        ',',
        till_comma.map(|s| unsafe { std::str::from_utf8_unchecked(s).trim() }),
        ',',
        parse_time,
        IFF_NEWLINE,
    )
        .map(|seq| Company {
            id: seq.0,
            code: seq.2.to_owned().into_boxed_str(),
            name: seq.4.to_owned().into_boxed_str(),
            end_of_timetable: seq.6,
        })
        .parse_next(input)
}

pub fn parse_company_file(
    input: Stream,
) -> Result<CompanyFile, ParseError<Stream, winnow::error::ContextError>> {
    (parse_header, repeat(0.., parse_company))
        .map(|seq| CompanyFile {
            header: seq.0,
            companies: seq.1,
        })
        .parse(input)
}
