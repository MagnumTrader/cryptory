use super::{
    period::{DateIterator, PeriodName},
    DateHelper, FormattedDate, Period,
};

use crate::{ticker::Tickerator, Ticker, TimeFrame};

use reqwest::Url;

use std::path::PathBuf;

/// The fileInfoIterator is used to iterate over the files
/// and urls that should be downloaded from binance.
#[derive(Debug)]
pub struct FileInfoIterator {
    period: Period,
    ticker_iter: Tickerator,
    curr_ticker: Option<Ticker>,
    date_iter: DateIterator,
    timeframe: TimeFrame,
    curr_id: usize,
}

impl Iterator for FileInfoIterator {
    type Item = FileInfo;

    fn next(&mut self) -> Option<Self::Item> {
        let ticker = self.curr_ticker.as_mut()?;

        let curr_date = if let Some(curr_date) = self.date_iter.next() {
            curr_date
        } else {
            *ticker = self.ticker_iter.next()?;
            self.date_iter.reset();
            self.date_iter.next().expect("we just reset the date_iter.")
        };

        let formatted_date = curr_date.date_url_str(&self.period);
        let period_name = self.period.period_name();

        let file_id = self.curr_id;
        self.curr_id += 1;

        Some(FileInfo::new(
            &ticker,
            &self.timeframe,
            period_name,
            formatted_date,
            file_id,
        ))
    }
}

impl FileInfoIterator {
    pub fn new(tickers: Vec<Ticker>, timeframe: TimeFrame, period: Period) -> FileInfoIterator {
        let date_iter = DateIterator::from(period.clone());
        let mut ticker_iter = Tickerator::from(tickers);
        let curr_ticker = ticker_iter.next();

        Self {
            period,
            timeframe,
            ticker_iter,
            curr_ticker,
            date_iter,
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
    pub fn file_name(&self) -> String {
        self.file_path
            .file_stem()
            .expect("we expect file_path to be a file")
            .to_str()
            .expect("valid str")
            .to_string()
    }
}
