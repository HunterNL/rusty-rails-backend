use std::fs::File;

use clap::builder::Str;

use crate::{iff, AppConfig};

pub(crate) fn start(config: AppConfig) -> Result<(), String> {
    let iff_file =
        File::open(config.cache_dir.join("ns-latest.zip")).expect("To find timetable file");

    let timetable = iff::parsing::Iff::from_file(&iff_file).unwrap();

    println!("{}", timetable.header.company_id);
    println!("{}", timetable.header.first_valid_date);
    println!("{}", timetable.header.last_valid_date);

    let a: Vec<String> = timetable
        .rides
        .iter()
        .map(|ride| ride.timetable.first().unwrap().code.clone())
        .collect();

    println!("{}", a.join(","));

    Ok(())
}
