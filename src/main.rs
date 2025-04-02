mod fetch;
use fetch::{FileInfo, FileInfoIterator, Period, TimeFrame};

use clap::Parser;
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;

use std::{fmt::Display, str::FromStr};

#[tokio::main]
async fn main() {
    let input = Input::parse();

    let client = reqwest::Client::new();
    let mut set = tokio::task::JoinSet::new();

    for fileinfo in input.to_fileinfo_iter() {

        let local_client = client.clone();
        set.spawn(async move {

            let FileInfo {
                source_url,
                file_path,
            } = fileinfo;
            let file_name = file_path
                .file_stem()
                .expect("we expect a file stem")
                .to_str()
                .unwrap()
                .to_string();

            println!("Downloading of {} started...", file_name);

            let request = local_client.get(source_url).send().await.unwrap();
            if !request.status().is_success() {
                eprintln!(
                    "{}: Make sure your ticker and date is valid!",
                    request.status()
                );
                std::process::exit(1)
            }

            // FIXME: this needs to be handled if we should overwrite or not
            let mut file = tokio::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open(file_path)
                .await
                .unwrap();

            let mut stream = request.bytes_stream();
            while let Some(Ok(item)) = stream.next().await {
                match file.write(&item).await {
                    Ok(bytes) => println!("wrote bytes {bytes} to: {file_name}"), // this should send it back for display
                    Err(e) => {
                        eprintln!("{e}: failed do write to file {}. Aborting", file_name);
                        break;
                    }
                }
            }

            // match tokio::io::copy(&mut reader, &mut writer).await {
            //     Ok(bytes_read) => println!("Successfully downloaded file, bytes_read: {bytes_read}"),
            //     Err(e) => eprintln!("ERROR: {e}"),
            // }
            println!("Done downloading {file_name}!")
        });
    }
    set.join_all().await;
}

#[derive(Debug, Parser)]
#[command(version, long_about = None)]
#[command(
    about = "Non official CLI for Binance public data\n\nMore information can be found on https://github.com/binance/binance-public-data/"
)]
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
