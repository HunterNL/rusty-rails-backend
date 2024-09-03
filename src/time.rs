use chrono::{NaiveDate, NaiveTime};

pub fn timetable_now() -> chrono::DateTime<chrono_tz::Tz> {
    let timetable_tz = chrono_tz::Europe::Amsterdam;
    chrono::Utc::now().with_timezone(&timetable_tz)
}
