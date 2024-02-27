use std::{cmp, error::Error, fmt::Display, str::FromStr};

use chrono::{NaiveTime, Timelike};
use serde::Serialize;

const MILLISECOND: u32 = 1;
const SECOND: u32 = MILLISECOND * 1000;
const MINUTE: u32 = SECOND * 60;
const HOUR: u32 = MINUTE * 60;

/**
Time as an offset from midnight.
Does not keep any date information.
Offset might overflow into next day.
Precision in milliseconds.
*/
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize)]
#[serde(transparent)]
pub struct DayOffset {
    offset: u32,
}

impl Ord for DayOffset {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.offset.cmp(&other.offset)
    }
}

impl PartialOrd for DayOffset {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl DayOffset {
    pub fn from_hour_minute(hours: u32, minutes: u32) -> Self {
        Self {
            offset: hours * HOUR + minutes * MINUTE,
        }
    }

    pub fn from_naivetime(time: &NaiveTime) -> Self {
        Self::from_hour_minute(time.hour(), time.minute())
    }

    pub fn offset_by(&self, minutes: i32) -> Self {
        Self {
            offset: self.offset.saturating_add_signed(minutes),
        }
    }
}

#[derive(Debug)]
pub enum ParseError {
    StringSizeInvalid,
    SubsliceParseFailed,
}

impl Error for ParseError {}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StringSizeInvalid => f.write_str("String size invalid, should be 4"),
            Self::SubsliceParseFailed => f.write_str("Subslice failed"),
        }
    }
}

impl FromStr for DayOffset {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 4 {
            Err(ParseError::StringSizeInvalid)?;
        }

        let hours: u32 = value[0..2]
            .parse()
            .map_err(|_| ParseError::SubsliceParseFailed)?;

        let minutes: u32 = value[2..4]
            .parse()
            .map_err(|_| ParseError::SubsliceParseFailed)?;

        Ok(DayOffset::from_hour_minute(hours, minutes))
    }
}
