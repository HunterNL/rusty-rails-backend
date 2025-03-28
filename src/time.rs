const TIMETABLE_TZ: chrono_tz::Tz = chrono_tz::Europe::Amsterdam;
pub fn timetable_now() -> chrono::DateTime<chrono_tz::Tz> {
    chrono::Utc::now().with_timezone(&TIMETABLE_TZ)
}

pub fn timetable_now_naive() -> chrono::NaiveDateTime {
    chrono::Utc::now()
        .with_timezone(&TIMETABLE_TZ)
        .naive_local()
}
