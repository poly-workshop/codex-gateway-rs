use chrono::{NaiveDate, Utc};

pub fn today() -> NaiveDate {
    Utc::now().date_naive()
}
