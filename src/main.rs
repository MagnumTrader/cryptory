mod fetch;
mod ticker;
mod user_input;

use fetch::*;
pub use ticker::Ticker;
use user_input::*;

mod progress_bars;
use progress_bars::ProgressBars;

use clap::Parser;
use tokio::sync::mpsc;

use std::io::ErrorKind;

#[tokio::main]
async fn main() {
    let input = Input::parse();

    if input.ticker.is_empty() {
        eprintln!("ERROR: you must provide atleast one ticker");
        std::process::exit(1)
    }

    let overwrite = input.overwrite;
    let mut rx = download_files(FileInfoIterator::from(input), overwrite);

    loop {
        if let Err(errors) = handle_file_updates(rx, true).await {
            println!("\nDone downloading files, but errors occured:");
            let mut need_overwrite = false;

            for (fileinfo, e) in errors.iter() {
                let extra = match e {
                    Error::CouldNotOpenFile(ErrorKind::AlreadyExists) => {
                        need_overwrite = true;
                        " -o to overwrite"
                    }
                    Error::CouldNotFindFileAtHost => " Is symbol and date correct?",
                    _ => "",
                };
                eprintln!("{} Failed with error: {e}.{extra}", fileinfo.file_name());
            }

            write_to_user("Do you want to retry downloading the other files that failed, y/n? ")
                .await;
            match user_input_yes_or_no().await {
                UserInput::Yes => {
                    if need_overwrite {
                        write_to_user("You have some files that already exists\nWould you like to overwrite existing files, y/n? ").await;
                        need_overwrite = loop {
                            match user_input_yes_or_no().await {
                                UserInput::Yes => break true,
                                UserInput::No => break false,
                                _ => {
                                    write_to_user("Invalid input, use y/n or yes/no").await;
                                    continue;
                                }
                            }
                        }
                    };

                    // Filter out the files that had an overwrite error
                    // if to_overwrite is false, and retry the files that had other errors.
                    let overwrite_filter = |(_, e): &(FileInfo, Error)| match e {
                        Error::CouldNotOpenFile(ErrorKind::AlreadyExists) => need_overwrite,
                        _ => true,
                    };
                    rx = download_files(
                        errors.into_iter().filter(overwrite_filter).map(|(x, _)| x),
                        need_overwrite,
                    );
                    continue;
                }
                UserInput::InvalidInput => break,
                UserInput::NotExpectedInput => break,
                UserInput::No => break,
            }
        } else {
            println!("Done downloading files!");
            break;
        }
    }
}

#[derive(Debug, Parser)]
#[command(
    about = "\nCryptory\nUnofficial CLI for Binance public data\nMore information can be found on https://github.com/binance/binance-public-data/"
)]
struct Input {
    /// The ticker symbol you want to fetch data for.
    ticker: Vec<Ticker>,

    /// The timeframe of the bars to fetch
    #[arg(short, long)]
    timeframe: TimeFrame,

    /// Period of the fetched file.
    #[command(subcommand)]
    period: Period,

    /// Force overwriting of files if they already exist
    #[arg(short, long)]
    overwrite: bool,
}

impl From<Input> for FileInfoIterator {
    fn from(value: Input) -> Self {
        FileInfoIterator::new(value.ticker, value.timeframe, value.period)
    }
}

type FileProgressReciever = mpsc::UnboundedReceiver<Msg>;

fn download_files(
    fileinfo_iter: impl Iterator<Item = FileInfo>,
    overwrite: bool,
) -> FileProgressReciever {
    let (tx, file_progress_rx) = mpsc::unbounded_channel::<Msg>();

    let client = reqwest::Client::new();

    for fileinfo in fileinfo_iter {
        tokio::spawn(crate::fetch::download_file(
            fileinfo,
            client.clone(),
            tx.clone(),
            overwrite,
        ));
    }

    file_progress_rx
}

async fn handle_file_updates(
    mut rx: FileProgressReciever,
    progress_bars: bool,
) -> Result<(), Vec<(FileInfo, Error)>> {
    let mut errors = Vec::new();

    if progress_bars {
        let mut bars = ProgressBars::new();
        while let Some(Msg { file_id, msg_type }) = rx.recv().await {
            match msg_type {
                MsgType::Written { bytes } => bars.increment(file_id, bytes),
                MsgType::Done => bars.finish(file_id, None::<String>),
                MsgType::Starting { total_size, name } => bars.new_bar(file_id, name, total_size),
                MsgType::Error { error, fileinfo } => {
                    errors.push((fileinfo, error));
                    bars.abandon(file_id, Some("ERROR"));
                }
            }
        }
    } else {
        while let Some(Msg { msg_type, .. }) = rx.recv().await {
            if let MsgType::Error { fileinfo, error } = msg_type {
                errors.push((fileinfo, error));
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    };

    Ok(())
}
