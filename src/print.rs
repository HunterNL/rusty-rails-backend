use anyhow::anyhow;
use chrono::Duration;

use crate::{
    api::datarepo::{self, DataRepo},
    cli,
    dayoffset::DayOffset,
    time, AppConfig,
};

pub fn print(config: &AppConfig, args: cli::PrintStruct) -> Result<(), anyhow::Error> {
    let data = datarepo::DataRepo::new(&config.cache_dir);

    match args.command {
        cli::PrintSubCommand::Departures { station } => {
            print_departures(&data, station.as_str()).map_err(|a| anyhow!(a))
        }
    }
}

// fn select_station_by_name<'a,'b,'c>(stations: &'a [&'c Station], name_or_code: &'b str) -> Option<&'a Station> {

// }

fn print_departures(data: &DataRepo, name_or_code: &str) -> Result<(), String> {
    let station = data
        .stations()
        .iter()
        .find(|station| station.code == name_or_code)
        .or_else(|| datarepo::select_station_by_name(data.stations(), name_or_code))
        .ok_or("failed to find station")?;

    let code = &station.code;

    println!("{}", station.name);

    let now = time::timetable_now();
    let future = now + Duration::hours(2); // TODO max ride time instead
    let mut active_rides =
        data.rides_active_in_timespan(&now.time(), &future.time(), &now.date_naive());

    active_rides.retain(|ride| ride.boardable_at_code(code));

    // Timestamp before which are hide departures, since they're too far in the past to be relevant
    let cutoff_time_start = DayOffset::from_naivetime(&now.time());
    let cutoff_time_end = cutoff_time_start.offset_by(2 * 60);

    // Match ride with their stop at the given station code
    // And filter these to trains that depart between `cutoff_time_start` and `cutoff_time_end`
    let mut ride_and_stop: Vec<_> = active_rides
        .into_iter()
        .map(|ride| (ride, ride.stop_at_code(code).unwrap()))
        .filter(|(_, stop)| {
            stop.stop_kind.departure_time().unwrap() > &cutoff_time_start
                && stop.stop_kind.departure_time().unwrap() < &cutoff_time_end
        })
        .collect();

    ride_and_stop.sort_by_key(|a| a.1.stop_kind.departure_time().unwrap());

    for (ride, stop) in ride_and_stop {
        println!(
            "{:5} {} {}",
            ride.id,
            stop.stop_kind
                .departure_time()
                .unwrap()
                .display_for_timetable(),
            data.station_by_code(&ride.timetable.last().unwrap().code)
                .unwrap()
                .name,
        )
    }

    Ok(())
}
