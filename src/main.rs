mod fetch;
use fetch::{FileInfo, FileInfoIterator, Period, TimeFrame};

mod progress_bars;
use progress_bars::ProgressBars;

use clap::Parser;
use futures_util::StreamExt;
use tokio::{io::AsyncWriteExt, sync::mpsc};

use std::{fmt::Display, str::FromStr};

// TODO: fetching Multiple symbols at once
// TODO: symbols from file
// TODO: remove files on interrupt?

#[tokio::main]
async fn main() {
    let input = Input::parse();
    let client = reqwest::Client::new();

    let (tx, mut file_progress_rx) = mpsc::unbounded_channel::<Msg>();
    let overwrite = input.overwrite;

    for fileinfo in FileInfoIterator::from(input) {
        tokio::spawn(download_file(
            fileinfo,
            client.clone(),
            tx.clone(),
            overwrite,
        ));
    }

    // Dropping tx to make sure main thread doesn't deadlock
    // waiting for this handle to send or drop
    drop(tx);

    let mut errors = Vec::new();
    let mut bars = ProgressBars::new();

    while let Some(Msg { file_id, msg_type }) = file_progress_rx.recv().await {
        match msg_type {
            MsgType::Written { bytes } => bars.increment(file_id, bytes),
            MsgType::Done => bars.finish(file_id, None::<String>),
            MsgType::Starting { total_size, name } => bars.new_bar(file_id, name, total_size),
            MsgType::Error { error, name } => {
                errors.push((name, error));
                bars.abandon(file_id, Some("ERROR"));
            }
        }
    }

    print!("Done downloading files");
    if !errors.is_empty() {
        print!(", but Errors occured!\n");
        for (name, e) in errors {
            eprintln!("{name} Failed with error: {e}");
        }
    } else {
        print!("!")
    }
}

#[derive(Debug)]
pub enum Error {
    FailedToSendRequest,
    CouldNotFindFileAtHost,
    CouldNotOpenFile(std::io::ErrorKind),
    FailedToWriteToFile,
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

async fn download_file(
    fileinfo: FileInfo,
    local_client: reqwest::Client,
    local_tx: mpsc::UnboundedSender<Msg>,
    overwrite: bool,
) {
    let file_name = fileinfo.file_name();
    let send_msg = move |msg: MsgType| {
        let _ = local_tx.send(Msg::new(fileinfo.file_id, msg));
    };

    let Ok(request) = local_client.get(fileinfo.source_url).send().await else {
        send_msg(MsgType::Error {
            name: file_name.clone(),
            error: Error::FailedToSendRequest,
        });
        return;
    };

    if !request.status().is_success() {
        send_msg(MsgType::Error {
            name: file_name.clone(),
            error: Error::CouldNotFindFileAtHost,
        });
        return;
    }

    let mut file = match open_file(fileinfo.file_path, overwrite).await {
        Ok(file) => file,
        Err(e) => {
            send_msg(MsgType::Error {
                name: file_name.clone(),
                error: Error::CouldNotOpenFile(e.kind()),
            });
            return;
        }
    };

    let total_size = request.content_length();
    send_msg(MsgType::Starting {
        name: file_name.clone(),
        total_size,
    });

    let mut stream = request.bytes_stream();

    while let Some(Ok(item)) = stream.next().await {
        let Ok(bytes) = file.write(&item).await else {
            send_msg(MsgType::Error {
                name: file_name.clone(),
                error: Error::FailedToWriteToFile,
            });
            return;
        };

        send_msg(MsgType::Written {
            bytes: bytes as u64,
        })
    }

    send_msg(MsgType::Done);
}

struct Msg {
    file_id: usize,
    msg_type: MsgType,
}

impl Msg {
    fn new(file_id: usize, msg_type: MsgType) -> Self {
        Self { file_id, msg_type }
    }
}

#[derive(Debug)]
enum MsgType {
    Error {
        name: String,
        error: Error,
    },
    Starting {
        name: String,
        total_size: Option<u64>,
    },
    Written {
        bytes: u64,
    },
    Done,
}

#[derive(Debug, Parser)]
#[command(
    about = "\n\nUnofficial CLI for Binance public data\nMore information can be found on https://github.com/binance/binance-public-data/"
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
    overwrite: bool,
) -> Result<tokio::fs::File, std::io::Error> {
    let mut open_options = tokio::fs::OpenOptions::new();

    if !overwrite {
        // create is ignored when create new is set.
        // so we can always include it.
        open_options.create_new(true);
    }
    open_options.create(true).write(true).open(path).await
}
