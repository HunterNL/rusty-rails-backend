use std::{error::Error, fmt::Display, str::FromStr};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct DayOffset {
    offset: u32,
}

impl DayOffset {
    pub fn from_hour_minute(hours: u32, minutes: u32) -> Self {
        DayOffset {
            offset: hours * 60 + minutes,
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
            ParseError::StringSizeInvalid => f.write_str("String size invalid, should be 4"),
            ParseError::SubsliceParseFailed => f.write_str("Subslice failed"),
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

        Ok(DayOffset {
            offset: hours * 60 + minutes,
        })
    }
}
