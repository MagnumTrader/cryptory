mod fetch;
use fetch::{FileInfo, FileInfoIterator, Period, TimeFrame};

use clap::Parser;

use std::{
    fmt::Display,
    io::{BufReader, BufWriter},
    str::FromStr,
};

fn main() {
    let input = Input::parse();

    // Iterate over all fileinfo
    for fileinfo in input.to_fileinfo_iter() {
        let FileInfo {
            source_url,
            file_path,
        } = fileinfo;

        // it should pop up in a list
        println!("Downloading of {source_url} started...");

        // do we get some sizes?
        let request = reqwest::blocking::get(source_url).unwrap();
        println!("{:?}", request.content_length());

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
    fn to_fileinfo_iter(&self) -> FileInfoIterator {
        let curr_date = self.period.start_date();
        let end_date = self.period.end_date().unwrap_or(curr_date);
        FileInfoIterator::new(self, curr_date, end_date)
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
