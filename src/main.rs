#![allow(unused, unreachable_code)]

use chrono::{Days, NaiveDate};
use clap::{Command, Parser, ValueEnum};
use std::{
    io::{BufReader, BufWriter, Read, Write},
    str::FromStr,
};

// TODO: in the file header do we get information about how large the file is?
//       display this to the user and confirm if fetching big files

const D_FILE: &str =
    "https://data.binance.vision/data/spot/monthly/klines/BTCUSDT/1m/BTCUSDT-1m-2025-01.zip";

fn main() {
    let input = Input::parse();
    println!("{input:?}");

    let next_day = input.from.checked_add_days(Days::new(1)).unwrap();
    println!("{next_day:?}");
    // construct the file link

    return;
    // NOTE: cmd line to download all the files
    //
    //                                                                add headers to the file
    //                                                              v like time open high etc
    // JBTCUSDT 1m --monthly --start 2025-01 --end 2025-01 -unzip --header etc
    //                                                      ^unzip into a csv file
    // download fi
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open("./my_download.zip")
        .unwrap();
    let mut b_writer = BufWriter::new(file);

    let mut req = reqwest::blocking::get(D_FILE).unwrap();
    let mut buf = [0; 512];
    let mut s = String::new();
    let mut b_reader = BufReader::new(req);

    while let Ok(n) = b_reader.read(&mut buf) {
        if n == 0 {
            break;
        }
        b_writer.write(&buf[..n]);
    }
    let _ = b_writer.flush();
    println!("done downloading!")
}
// TODO: Construct a file name correctly
// TODO: Check if the filename is already there

// Application model
// TODO: Write docs for clap
//       - where to find info on the api, and where we are getting files from

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Input {
    /// The ticker symbol you want to fetch data for.
    ticker: Ticker,
    timeframe: TimeFrame,
    /// Period of the fetched file.
    period: Period,
    /// Select the first date you want data from.
    from: FromDate,

    /// Select the last date you want data to.
    /// If left out, assumes todays date.
    #[arg(short)]
    to: Option<ToDate>,
}

type Ticker = String;
type FromDate = NaiveDate;
type ToDate = NaiveDate;

#[derive(Debug, Clone, ValueEnum)]
enum Period {
    // format year-month YY-MM
    /// Fetch file with a whole month worth of data
    Monthly,
    //  format year-month-day YY-MM-DD
    /// Fetch each day in separate files
    Daily,
}

impl FromStr for Period {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "daily" | "d" => Ok(Period::Daily),
            "monthly" | "m" => Ok(Period::Monthly),
            _ => Err("Invalid Period, valid is daily, d, monthly, m"),
        }
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
