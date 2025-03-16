use chrono::NaiveDate;
use serde::{ser::SerializeStruct, Serialize};

use crate::{api::ApiObject, ride_recurrance::RideRecurrence};

#[derive(Serialize, Debug, Clone)]
pub struct Ride<'a> {
    pub date: NaiveDate,
    pub recurrence: &'a RideRecurrence,
}
