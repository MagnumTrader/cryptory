use crate::fetch::DateHelper;

use chrono::{Datelike, NaiveDate};
use clap::Subcommand;

use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug)]
pub struct DateIterator {
    next_date: Option<NaiveDate>,
    end_date: NaiveDate,
    period: Period,
}
impl DateIterator {
    pub fn reset(&mut self) {
        *self = DateIterator::from(self.period.clone())
    }
}

impl From<Period> for DateIterator {
    fn from(value: Period) -> Self {
        let start_date = value.start_date();
        let end_date = value.end_date().unwrap_or(start_date);

        DateIterator {
            next_date: Some(start_date),
            end_date,
            period: value,
        }
    }
}

impl Iterator for DateIterator {
    type Item = NaiveDate;

    fn next(&mut self) -> Option<Self::Item> {
        let current_date = self.next_date.take()?;

        if current_date < self.end_date {
            let next_date = current_date
                .add_date_from_period(&self.period)
                .expect("date out of bounds");

            self.next_date = Some(next_date);
        }

        Some(current_date)
    }
}

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

//TODO: Create better parsing for period so start_date >= end_date etc
pub fn parse_monthly(input: &str) -> Result<NaiveDate, &'static str> {
    let date = match NaiveDate::from_str(input) {

        // NOTE: this is to fix bug where we iterate months
        // when we check if current_date < end_date, this needs to 
        // be set the first in the month.
        Ok(date) => date.with_day(1).expect("valid day"),
        Err(_) => {
            let mut s: String = input.into();
            s.push_str("-01");
            NaiveDate::from_str(&s).map_err(|_| "Invalid date format")?
        }
    };
    Ok(date)
}

impl Period {
    #[allow(unused)]
    pub fn new(start_date: NaiveDate, end_date: Option<NaiveDate>, period: PeriodName) -> Self {
        match period {
            PeriodName::Daily => {
                let end_date = end_date.unwrap_or(start_date);
                Self::Daily {
                    start_date,
                    end_date: Some(end_date),
                }
            }
            PeriodName::Monthly => {
                let end_date = end_date
                    .unwrap_or(start_date)
                    .with_day(1)
                    .expect("1st is valid date");
                let start_date = start_date.with_day(1).expect("1st is valid date");
                Self::Monthly {
                    start_date,
                    end_date: Some(end_date),
                }
            }
        }
    }

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

#[cfg(test)]
mod tests {

    fn nd(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    use super::*;
    #[test]
    fn date_iter_daily() {
        let period = Period::new(nd(2025, 1, 1), Some(nd(2025, 1, 5)), PeriodName::Daily);

        let mut date_iter = DateIterator::from(period.clone());
        assert_eq!(Some(nd(2025, 1, 1)), date_iter.next());
        assert_eq!(Some(nd(2025, 1, 2)), date_iter.next());
        assert_eq!(Some(nd(2025, 1, 3)), date_iter.next());
        assert_eq!(Some(nd(2025, 1, 4)), date_iter.next());
        assert_eq!(Some(nd(2025, 1, 5)), date_iter.next());
        assert_eq!(None, date_iter.next());

        let period = Period::new(nd(2025, 1, 1), None, PeriodName::Daily);
        let mut date_iter = DateIterator::from(period.clone());
        assert_eq!(Some(nd(2025, 1, 1)), date_iter.next());
        assert_eq!(None, date_iter.next());
        assert_eq!(None, date_iter.next());
    }

    #[test]
    fn date_iter_monthly() {
        let period = Period::new(nd(2025, 1, 1), Some(nd(2025, 3, 6)), PeriodName::Monthly);

        println!("{period:?}");

        let mut date_iter = DateIterator::from(period.clone());

        assert_eq!(Some(nd(2025, 1, 1)), date_iter.next());
        assert_eq!(Some(nd(2025, 2, 1)), date_iter.next());
        assert_eq!(Some(nd(2025, 3, 1)), date_iter.next());
        assert_eq!(None, date_iter.next());

        let period = Period::new(nd(2025, 1, 1), Some(nd(2025, 1, 1)), PeriodName::Monthly);
        let mut date_iter = DateIterator::from(period.clone());
        assert_eq!(Some(nd(2025, 1, 1)), date_iter.next());
        assert_eq!(None, date_iter.next());
        assert_eq!(None, date_iter.next());
    }

    #[test]
    fn date_iter_reset() {
        let period = Period::new(nd(2025, 1, 1), Some(nd(2025, 1, 5)), PeriodName::Daily);

        let mut date_iter = DateIterator::from(period.clone());
        assert_eq!(Some(nd(2025, 1, 1)), date_iter.next());
        assert_eq!(Some(nd(2025, 1, 2)), date_iter.next());
        date_iter.reset();
        assert_eq!(Some(nd(2025, 1, 1)), date_iter.next());
        assert_eq!(Some(nd(2025, 1, 2)), date_iter.next());
        assert_eq!(Some(nd(2025, 1, 3)), date_iter.next());
        assert_eq!(Some(nd(2025, 1, 4)), date_iter.next());
        assert_eq!(Some(nd(2025, 1, 5)), date_iter.next());
        assert_eq!(None, date_iter.next());

        let period = Period::new(nd(2025, 1, 1), None, PeriodName::Daily);
        let mut date_iter = DateIterator::from(period.clone());
        assert_eq!(Some(nd(2025, 1, 1)), date_iter.next());
        assert_eq!(None, date_iter.next());
        date_iter.reset();
        assert_eq!(Some(nd(2025, 1, 1)), date_iter.next());
        assert_eq!(None, date_iter.next());
    }
}
