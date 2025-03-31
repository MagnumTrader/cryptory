#![allow(unused, unreachable_code)]
use chrono::{Datelike, NaiveDate};
use clap::{Parser, Subcommand, ValueEnum};
use reqwest::Url;
use std::{
    fmt::Display,
    io::{BufReader, BufWriter, Read, Write},
    str::FromStr,
};

fn main() {

    let input = Input::parse();
    let url = construct_file_url(&input.ticker, &input.timeframe, &input.period);

    //https://data.binance.vision/data/spot/monthly/klines/ETHUSDT/1m/ETHUSDT-1m-2025-01.zip
    let req = reqwest::blocking::get(url).unwrap();
    if !req.status().is_success() {
        eprintln!("{}: Make sure your ticker and date is valid!", req.status());
        std::process::exit(1)
    }

    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(format!(
            "./{}-{}-{}.zip",
            input.ticker,
            input.timeframe,
            input.period.start_date_url_string()
        ))
        .unwrap();

    let mut writer = BufWriter::new(file);
    let mut reader = BufReader::new(req);

    println!("Downloading has started...");
    match std::io::copy(&mut reader, &mut writer) {
        Ok(bytes_read) => println!("Successfully downloaded file, bytes_read: {bytes_read}"),
        Err(e) => eprintln!("ERROR: {e}"),
    }
    println!("Done downloading!")
}

// Valid url: "https://data.binance.vision/data/spot/monthly/klines/BTCUSDT/1m/BTCUSDT-1m-2025-01.zip";
fn construct_file_url(ticker: &Ticker, timeframe: &TimeFrame, period: &Period) -> Url {
    Url::parse(&format!("https://data.binance.vision/data/spot/{}/klines/{ticker}/{timeframe}/{ticker}-{timeframe}-{}.zip", period.period_name(), period.start_date_url_string()))
    .unwrap()
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

    fn start_date(&self) -> &NaiveDate {
        match self {
            Period::Daily { start_date, .. } => start_date,
            Period::Monthly { start_date, .. } => start_date,
        }
    }

    /// these may belong to the iterator
    fn start_date_url_string(&self) -> String {
        match self {
            Period::Daily { start_date, .. } => start_date.to_string(),
            Period::Monthly { start_date, .. } => {
                format!("{}-{}", start_date.year(), start_date.format("%m"))
            }
        }
    }

    fn end_date(&self) -> Option<&NaiveDate> {
        match self {
            Period::Daily { end_date, .. } => end_date.as_ref(),
            Period::Monthly { end_date, .. } => end_date.as_ref(),
        }
    }

    fn end_date_url_string(&self) -> Option<String> {
        match self {
            Period::Daily {
                end_date: Some(end_date),
                ..
            } => Some(end_date.to_string()),
            Period::Monthly {
                end_date: Some(end_date),
                ..
            } => {
                // >                    Format month to be 0x format   v
                let s = format!("{}-{}", end_date.year(), end_date.format("%m"));
                Some(s)
            }
            _ => None,
        }
    }

    fn period_str_iterator(&self) -> PeriodUrlIterator {
        todo!()
    }
}

impl Display for Period {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

struct PeriodUrlIterator {}
// TODO: IntoUrlIterator

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

type FromDate = NaiveDate;
type ToDate = NaiveDate;

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
