mod fetch;
use fetch::{FileInfo, FileInfoIterator, Period, TimeFrame};

use clap::Parser;
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;

use std::{fmt::Display, str::FromStr, sync::Arc};

// TODO: Openfile to function
// TODO: Permit for tasks, X at the time
// TODO: channel on main thread collects info,

#[tokio::main]
async fn main() {
    let input = Arc::new(Input::parse());
    let client = reqwest::Client::new();
    let mut set = tokio::task::JoinSet::new();

    for fileinfo in input.to_fileinfo_iter() {
        let local_client = client.clone();
        let local_input = Arc::clone(&input);

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

            let mut file = match open_file(file_path, &local_input).await {
                Ok(file) => file,
                Err(e) => {
                    eprintln!("error when opening {file_name}: {e}");
                    return;
                }
            };

            println!("Downloading of {} started...", file_name);
            let request = match local_client.get(source_url).send().await {
                Ok(req) => req,
                Err(e) => {
                    eprintln!("Failed to download file {file_name}. Error: {e}");
                    return;
                }
            };

            if !request.status().is_success() {
                eprintln!(
                    "{}: Make sure your ticker and date is valid!",
                    request.status()
                );
                std::process::exit(1)
            }

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
    #[arg(short, long)]
    overwrite: bool,
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
async fn open_file(
    path: std::path::PathBuf,
    input: &Input,
) -> Result<tokio::fs::File, std::io::Error> {
    let mut open_options = tokio::fs::OpenOptions::new();

    if !input.overwrite {
        // create is ignored when create new is set.
        // so we can always include it.
        open_options.create_new(true);
    }
    open_options.create(true).write(true).open(path).await
}
