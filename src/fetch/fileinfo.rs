use std::path::PathBuf;

use chrono::NaiveDate;
use reqwest::Url;

use crate::{Input, Ticker, TimeFrame};

use super::{period::PeriodName, DateHelper, FormattedDate};

/// The fileInfoIterator is used to iterate over the files
/// and urls that should be downloaded from binance
#[derive(Debug)]
pub struct FileInfoIterator<'a> {
    input: &'a Input,
    curr_date: NaiveDate,
    end_date: NaiveDate,
    curr_id: usize,
}

impl<'a> Iterator for FileInfoIterator<'a> {
    type Item = FileInfo;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr_date > self.end_date {
            return None;
        }

        let period = &self.input.period;
        // Maybe these should be types later?
        let formatted_date = self.curr_date.date_url_str(period);
        let period_name = period.period_name();

        self.curr_date = self
            .curr_date
            .add_date_from_period(&self.input.period)
            .expect("expect valid date range");

        let file_id = self.curr_id;
        self.curr_id += 1;

        Some(FileInfo::new(
            &self.input.ticker,
            &self.input.timeframe,
            period_name,
            formatted_date,
            file_id,
        ))
    }
}
impl<'a> FileInfoIterator<'a> {
    pub fn new(
        input: &'a crate::Input,
        curr_date: NaiveDate,
        end_date: NaiveDate,
    ) -> FileInfoIterator<'a> {
        Self {
            input,
            curr_date,
            end_date,
            curr_id: 1,
        }
    }
}

#[derive(Debug)]
pub struct FileInfo {
    pub source_url: Url,
    pub file_path: PathBuf,
    pub file_id: usize,
}

impl FileInfo {
    pub fn new(
        ticker: &Ticker,
        timeframe: &TimeFrame,
        period_name: PeriodName,
        formatted_date: FormattedDate,
        file_id: usize,
    ) -> Self {
        let file_name = format!("{ticker}-{timeframe}-{formatted_date}.zip");
        let url_str = format!("https://data.binance.vision/data/spot/{period_name}/klines/{ticker}/{timeframe}/{file_name}");

        let source_url = Url::parse(&url_str).expect("expect correct url format above");

        let mut file_path = PathBuf::from(std::env::current_dir().unwrap());
        file_path.push(file_name);

        FileInfo {
            source_url,
            file_path,
            file_id,
        }
    }
}
