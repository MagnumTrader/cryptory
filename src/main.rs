use chrono::NaiveDate;
use clap::{Parser, Subcommand};
use reqwest::Url;
use std::{
    fmt::Display,
    io::{BufReader, BufWriter},
    path::PathBuf,
    str::FromStr,
};

fn main() {
    let input = Input::parse();
    let mut fileinfo_iter = input.to_url_iter();

    // Iterate over all fileinfo
    while let Some(FileInfo {
        source_url,
        file_path,
    }) = fileinfo_iter.next()
    {
        println!("Downloading of {source_url} started...");
        let request = reqwest::blocking::get(source_url).unwrap();
        if !request.status().is_success() {
            eprintln!(
                "{}: Make sure your ticker and date is valid!",
                request.status()
            );
            std::process::exit(1)
        }

        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(file_path)
            .unwrap();

        let mut reader = BufReader::new(request);
        let mut writer = BufWriter::new(file);

        match std::io::copy(&mut reader, &mut writer) {
            Ok(bytes_read) => println!("Successfully downloaded file, bytes_read: {bytes_read}"),
            Err(e) => eprintln!("ERROR: {e}"),
        }
        println!("Done downloading!")
    }
}

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Input {
    /// The ticker symbol you want to fetch data for.
    ticker: Ticker,
    /// The timeframe of the bars to fetch
    timeframe: TimeFrame,
    /// Period of the fetched file.
    #[command(subcommand)]
    period: Period,
}

impl Input {
    fn to_url_iter(&self) -> FileInfoIterator {
        let curr_date = self.period.start_date();
        let end_date = self.period.end_date().unwrap_or(curr_date);

        FileInfoIterator {
            curr_date,
            end_date,
            input: self,
        }
    }
}

/// The file info iterator is used to iterate over the files and urls that is to be downloaded.
#[derive(Debug)]
struct FileInfoIterator<'a> {
    input: &'a Input,
    curr_date: NaiveDate,
    end_date: NaiveDate,
}

impl<'a> Iterator for FileInfoIterator<'a> {
    type Item = FileInfo;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr_date > self.end_date {
            return None;
        }

        let period = &self.input.period;
        let formatted_date = self.curr_date.date_url_str(period);
        let period_name = period.period_name();

        self.curr_date = self
            .curr_date
            .add_date_from_period(&self.input.period)
            .expect("expect valid date range");

        Some(FileInfo::new(
            &self.input.ticker,
            &self.input.timeframe,
            period_name,
            &formatted_date,
        ))
    }
}

#[derive(Debug)]
struct FileInfo {
    source_url: Url,
    file_path: PathBuf,
}

impl FileInfo {
    fn new(
        ticker: &Ticker,
        timeframe: &TimeFrame,
        period_name: &str,
        formatted_date: &str,
    ) -> Self {
        let file_name = format!("{ticker}-{timeframe}-{formatted_date}.zip");
        let url_str = format!("https://data.binance.vision/data/spot/{period_name}/klines/{ticker}/{timeframe}/{file_name}");

        let source_url = Url::parse(&url_str).expect("expect correct url");

        let mut file_path = PathBuf::from(std::env::current_dir().unwrap());
        file_path.push(file_name);

        FileInfo {
            source_url,
            file_path,
        }
    }
}

/// Period of the fetched file.
#[derive(Debug, Clone, Subcommand)]
enum Period {
    /// Fetch file(s) for each day in the period from start to end date.
    Daily {
        /// Select the first date you want data from.
        start_date: NaiveDate,
        /// Select the last date you want data to.
        /// If left out, will only download the day of start_date
        #[arg(short)]
        end_date: Option<NaiveDate>,
    },
    // TODO: implement parsing for Monthly so we can pass 2025-01 only
    /// Fetch file(s) for each month in the period from start to end date.
    /// Daily numbers will be ignored, only year and month is taken into account.
    Monthly {
        /// Select the first date you want data from.
        /// Will only use the year and month part.
        start_date: NaiveDate,
        /// Select the last date you want data to.
        /// If left out, will only download month of start_date
        #[arg(short)]
        end_date: Option<NaiveDate>,
    },
}

impl Period {
    #[inline]
    fn period_name(&self) -> &str {
        match self {
            Period::Daily { .. } => "daily",
            Period::Monthly { .. } => "monthly",
        }
    }

    fn start_date(&self) -> NaiveDate {
        match self {
            Period::Daily { start_date, .. } => *start_date,
            Period::Monthly { start_date, .. } => *start_date,
        }
    }

    fn end_date(&self) -> Option<NaiveDate> {
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

trait DateHelper: Sized {
    fn add_date_from_period(&self, period: &Period) -> Option<Self>;
    fn date_url_str(&self, period: &Period) -> String;
}

impl DateHelper for NaiveDate {
    fn add_date_from_period(&self, period: &Period) -> Option<NaiveDate> {
        match period {
            Period::Daily { .. } => self.checked_add_days(chrono::Days::new(1)),
            Period::Monthly { .. } => self.checked_add_months(chrono::Months::new(1)),
        }
    }

    fn date_url_str(&self, period: &Period) -> String {
        match period {
            Period::Daily { .. } => self.to_string(),
            Period::Monthly { .. } => self.format("%Y-%m").to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct Ticker(String);

impl Display for Ticker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Ticker {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // validate symbols here i guess url will fail?
        Ok(Self(s.to_uppercase()))
    }
}

/// Valid representation of timeframes that can be fetched from binance.
#[derive(Debug, Clone)]
struct TimeFrame(String);

impl FromStr for TimeFrame {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "12h" | "15m" | "1d" | "1h" | "1m" | "1mo" | "1s" | "1w" | "2h" | "30m" | "3d"
            | "3m" | "4h" | "5m" | "6h" | "8h" => Ok(TimeFrame(s.to_string())),
            _ => Err("Invalid timeframe"),
        }
    }
}

impl Display for TimeFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
