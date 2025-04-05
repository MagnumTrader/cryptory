use std::fmt::Display;
use std::str::FromStr;

use chrono::NaiveDate;
use clap::Subcommand;

/// Period of the fetched file.
/// Format:
/// 2025-01-01 for Daily
/// 2025-01 or 2025-01-01 for Monthly (date will be ignored)
#[derive(Debug, Clone, Subcommand)]
pub enum Period {
    /// Fetch file(s) for each day in the period from start to end date.
    Daily {
        /// Select the first date you want data from.
        start_date: NaiveDate,
        /// Select the last date you want data to.
        /// If left out, will only download the day of start_date
        #[arg(short)]
        end_date: Option<NaiveDate>,
    },
    /// Fetch file(s) for each month in the period from start to end date.
    Monthly {
        /// Select the first date you want data from.
        #[arg(value_parser = parse_monthly)]
        start_date: NaiveDate,
        /// Select the last date you want data to.
        /// If left out, will only download month of start_date
        #[arg(short, value_parser = parse_monthly)]
        end_date: Option<NaiveDate>,
    },
}

fn parse_monthly(input: &str) -> Result<NaiveDate, &'static str> {
    let date = match NaiveDate::from_str(input) {
        Ok(date) => date,
        Err(_) => {
            let mut s: String = input.into();
            s.push_str("-01");
            NaiveDate::from_str(&s).map_err(|_| "Invalid date format")?
        }
    };
    Ok(date)
}

impl Period {
    #[inline]
    pub fn period_name(&self) -> PeriodName {
        match self {
            Period::Daily { .. } => PeriodName::Daily,
            Period::Monthly { .. } => PeriodName::Monthly,
        }
    }

    pub fn start_date(&self) -> NaiveDate {
        match self {
            Period::Daily { start_date, .. } => *start_date,
            Period::Monthly { start_date, .. } => *start_date,
        }
    }

    pub fn end_date(&self) -> Option<NaiveDate> {
        match self {
            Period::Daily { end_date, .. } => *end_date,
            Period::Monthly { end_date, .. } => *end_date,
        }
    }
}

impl Display for Period {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub enum PeriodName {
    Daily,
    Monthly,
}

impl Display for PeriodName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            PeriodName::Daily => "daily",
            PeriodName::Monthly => "monthly",
        };
        write!(f, "{}", s)
    }
}
