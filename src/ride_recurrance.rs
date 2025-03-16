use std::fmt::{Display, Write};

use serde::Serialize;

use crate::{
    dayoffset::DayOffset,
    iff::{
        timetable::{generate_legs, timetable_end, timetable_start, TimetableEntry},
        Leg, LocationCache, LocationCodeHandle,
    },
};

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct RideRecurrence {
    pub id: String,
    pub transit_mode: String,
    pub timetable: Vec<TimetableEntry>,
    pub day_validity: u64,
    pub previous: Option<String>,
    pub next: Option<String>,
    pub operator: u32,
}

pub struct RidePrettyPrint<'a>(&'a RideRecurrence, &'a LocationCache);

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

impl RideRecurrence {
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

    pub fn departure_time(&self) -> DayOffset {
        self.start_time()
    }

    pub fn arrival_time(&self) -> DayOffset {
        self.end_time()
    }
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
