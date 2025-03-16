use serde::Serialize;

use crate::{dayoffset::DayOffset, iff::StopKind};

use super::{Leg, LegKind, LocationCache, LocationCodeHandle};

pub fn timetable_stop_index(entries: &[TimetableEntry], nth: usize) -> Option<usize> {
    entries
        .iter()
        .enumerate()
        .filter(|(_, stop)| !stop.stop_kind.is_waypoint())
        .nth(nth)
        .map(|(index, _)| index)
}

pub fn timetable_start(entries: &[TimetableEntry]) -> DayOffset {
    *entries
        .first()
        .expect("timetable to have an entry")
        .stop_kind
        .departure_time()
        .expect("first entry to have a departure time")
}

pub fn timetable_end(entries: &[TimetableEntry]) -> DayOffset {
    *entries
        .last()
        .expect("timetable to have an entry")
        .stop_kind
        .arrival_time()
        .expect("last entry to have an arrival time")
}

pub fn timetable_normalize_ends(entries: &mut [TimetableEntry]) {
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
pub struct TimetableEntryRaw<'a> {
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
/// Turn a slice of TimetableEntry's into Legs
/// This process collects ajoining waypoints into MovingLegs
pub fn generate_legs(entries: &[TimetableEntry]) -> Vec<Leg> {
    let mut out = vec![];
    let mut waypoints = vec![];
    let first_stop = entries.first().expect("timetable to have an entry");
    let mut previous_stop = first_stop;

    out.push(leg_for_stop(first_stop));

    entries.iter().skip(1).for_each(|entry| {
        // Collect non-stopping points into waypoints.
        // These are needed later on to find the right Links between Stations
        if entry.stop_kind.is_waypoint() {
            waypoints.push(entry);
            return;
        }

        out.push(Leg {
            start: leg_for_stop(previous_stop).end,
            end: leg_for_stop(entry).start,
            kind: LegKind::Moving {
                from: previous_stop.code,
                to: entry.code,
                waypoints: waypoints.iter().map(|c| c.code).collect(),
            },
        });

        previous_stop = entry;

        waypoints.clear();

        out.push(leg_for_stop(entry));
    });

    out
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
        kind: LegKind::Stationary(entry.code, entry.stop_kind.clone()),
    }
}
