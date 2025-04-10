mod fetch;
mod ticker;
use fetch::{FileInfo, FileInfoIterator, Period, TimeFrame};

mod progress_bars;
use progress_bars::ProgressBars;

use clap::Parser;
use futures_util::StreamExt;
pub use ticker::Ticker;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc,
};

use std::{fmt::Display, io::ErrorKind};

// TODO: Cleaning
// TODO: Input to specific function Yes, no, other(String)

#[tokio::main]
async fn main() {
    let input = Input::parse();

    let overwrite = input.overwrite;
    let mut rx = download_files(FileInfoIterator::from(input), overwrite);

    loop {
        if let Err(errors) = handle_file_updates(rx, true).await {
            println!("Done downloading files, but errors occured:");
            let mut to_overwrite = false;

            for (fileinfo, e) in errors.iter() {
                let extra = match e {
                    Error::CouldNotOpenFile(ErrorKind::AlreadyExists) => {
                        to_overwrite = true;
                        " -o to overwrite"
                    }
                    Error::CouldNotFindFileAtHost => {
                        " Is symbol and date correct?"
                    }
                    _ => "",
                };
                eprintln!("{} Failed with error: {e}.{extra}", fileinfo.file_name());
            }


            write_to_user("Do you want to retry downloading these files, y/n? ").await;

            match user_input_yes_or_no().await {
                UserInput::Yes => {
                    // exclude overwriting always, and rely on the flag?
                    if to_overwrite {
                        write_to_user("You have some files that already exists\nWould you like to overwrite existing files, y/n? ").await;
                        match user_input_yes_or_no().await {
                            UserInput::Yes => to_overwrite = true,
                            UserInput::No => to_overwrite = false,
                            UserInput::NotExpectedInput => todo!(),
                            UserInput::InvalidInput => todo!(),
                        }
                    };

                    // Filter out the files that had an overwrite error
                    // if to_overwrite is false, and retry the files that had other errors.
                    let overwrite_filter = |(_, e): &(FileInfo, Error)| match e {
                        Error::CouldNotOpenFile(ErrorKind::AlreadyExists) => to_overwrite,
                        _ => true,
                    };

                    rx = download_files(
                        errors.into_iter().filter(overwrite_filter).map(|(x, _)| x),
                        to_overwrite,
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

// TODO: this should be a function part of getting the input aswell 
async fn write_to_user(s: &str)  {
    let mut stdout = tokio::io::stdout();
    let _ = stdout.write(s.as_bytes()).await;
    let _ = stdout.flush().await;
}


enum UserInput {
    Yes,
    No,
    NotExpectedInput,
    InvalidInput,
}

async fn take_user_input() -> Result<String, std::string::FromUtf8Error> {
    let mut user_input = [0; 128];
    let mut stdin = tokio::io::stdin();
    let bytes = stdin.read(&mut user_input).await.unwrap();
    let input = user_input[..bytes].trim_ascii_end();
    String::from_utf8(input.to_vec())
}

async fn user_input_yes_or_no() -> UserInput {
    let Ok(user_input) = take_user_input().await else {
        return UserInput::InvalidInput;
    };

    match user_input.as_str() {
        "y" | "Y" | "Yes" | "yes" | "YES" => UserInput::Yes,
        "n" | "N" | "No" | "no" | "NO" => UserInput::No,
        _ => UserInput::NotExpectedInput,
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
        tokio::spawn(download_file(
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
            match msg_type {
                MsgType::Error { error, fileinfo } => {
                    errors.push((fileinfo, error));
                }
                _ => {}
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
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

    let Ok(request) = local_client.get(fileinfo.source_url.clone()).send().await else {
        send_msg(MsgType::Error {
            fileinfo: fileinfo.clone(),
            error: Error::FailedToSendRequest,
        });
        return;
    };

    if !request.status().is_success() {
        send_msg(MsgType::Error {
            fileinfo,
            error: Error::CouldNotFindFileAtHost,
        });
        return;
    }

    let mut file = match open_file(fileinfo.file_path.clone(), overwrite).await {
        Ok(file) => file,
        Err(e) => {
            send_msg(MsgType::Error {
                fileinfo,
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
                fileinfo,
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
        fileinfo: FileInfo,
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
    // TODO: add alternative to not print, only print the downloaded files to stdout.
}

impl From<Input> for FileInfoIterator {
    fn from(value: Input) -> Self {
        FileInfoIterator::new(value.ticker, value.timeframe, value.period)
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
