mod fileinfo;
mod period;
mod timeframe;

pub use fileinfo::{FileInfo, FileInfoIterator};
pub use period::Period;
pub use timeframe::TimeFrame;

use chrono::NaiveDate;

use std::fmt::Display;

trait DateHelper: Sized {
    fn add_date_from_period(&self, period: &Period) -> Option<Self>;
    fn date_url_str(&self, period: &Period) -> FormattedDate;
}

impl DateHelper for NaiveDate {
    fn add_date_from_period(&self, period: &Period) -> Option<NaiveDate> {
        match period {
            Period::Daily { .. } => self.checked_add_days(chrono::Days::new(1)),
            Period::Monthly { .. } => self.checked_add_months(chrono::Months::new(1)),
        }
    }

    fn date_url_str(&self, period: &Period) -> FormattedDate {
        match period {
            Period::Daily { .. } => FormattedDate(self.to_string()),
            Period::Monthly { .. } => FormattedDate(self.format("%Y-%m").to_string()),
        }
    }
}

pub(super) struct FormattedDate(String);

impl Display for FormattedDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
