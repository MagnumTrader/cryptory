mod fetch;
use fetch::{FileInfo, FileInfoIterator, Period, TimeFrame};

use clap::Parser;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::{io::AsyncWriteExt, sync::mpsc};

use std::{collections::HashMap, fmt::Display, str::FromStr};

// TODO: Abstract progressbars handling
// TODO: fetching Multiple symbols at once
// TODO: Cleaner errors

#[tokio::main]
async fn main() {
    let input = Input::parse();
    let client = reqwest::Client::new();

    let (tx, mut file_progress_rx) = mpsc::unbounded_channel::<Msg>();
    let overwrite = input.overwrite;

    // HACK: i dont like this abstract or remove later
    let mut bars: HashMap<usize, (String, Option<ProgressBar>)> = HashMap::new();

    for fileinfo in input.to_fileinfo_iter() {
        bars.insert(fileinfo.file_id, (fileinfo.file_name(), None));
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
    let style = ProgressStyle::with_template(
        "{msg} {bar:30.cyan/blue} {decimal_bytes:>7}/{decimal_total_bytes}",
    )
    .unwrap();

    let mpg = indicatif::MultiProgress::new();

    while let Some(msg) = file_progress_rx.recv().await {
        let Msg { file_id, msg_type } = msg;
        match msg_type {
            MsgType::Written { bytes } => expect_progress_bar(&bars, file_id).inc(bytes),
            MsgType::Done => expect_progress_bar(&bars, file_id).finish_with_message("Done"),
            MsgType::Starting { total_size } => {
                let name = expect_file_name(&bars, file_id);
                let pb = if let Some(total_size) = total_size {
                    ProgressBar::new(total_size)
                        .with_style(style.clone().progress_chars("##-"))
                        .with_message(name.to_string())
                } else {
                    let spinner = ProgressBar::new_spinner();
                    spinner.enable_steady_tick(std::time::Duration::from_millis(200));
                    spinner
                };
                let pb = mpg.add(pb);
                if let Some((_, opt)) = bars.get_mut(&file_id) {
                    let _ = opt.insert(pb);
                } else {
                    unreachable!("we should have a file with that id at this point")
                }
            }
            MsgType::Error(e) => {
                errors.push(e);
                if let Some((_, Some(bar))) = bars.get(&file_id) {
                    bar.abandon_with_message("ERROR")
                }
            }
        }
    }

    if errors.is_empty() {
        println!("Done downloading files!")
    } else {
        eprintln!("Errors occured!");
        for e in errors {
            eprintln!("{e}");
        }
    }
}

fn expect_file_id(
    bars: &HashMap<usize, (String, Option<ProgressBar>)>,
    file_id: usize,
) -> &(String, Option<ProgressBar>) {
    bars.get(&file_id).expect("expect file id")
}

fn expect_progress_bar(
    bars: &HashMap<usize, (String, Option<ProgressBar>)>,
    file_id: usize,
) -> &ProgressBar {
    expect_file_id(bars, file_id)
        .1
        .as_ref()
        .expect("we expect a bar to be present")
}

fn expect_file_name(bars: &HashMap<usize, (String, Option<ProgressBar>)>, file_id: usize) -> &str {
    expect_file_id(bars, file_id).0.as_str()
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

    let request = match local_client.get(fileinfo.source_url).send().await {
        Ok(req) => req,
        Err(e) => {
            let e = format!("Failed to download file {file_name}. Error: {e}");
            send_msg(MsgType::Error(e));
            return;
        }
    };

    if !request.status().is_success() {
        let e = format!(
            "{}: Make sure your ticker and date is valid!",
            request.status()
        );
        send_msg(MsgType::Error(e));
        return;
    }

    let mut file = match open_file(fileinfo.file_path, overwrite).await {
        Ok(file) => file,
        Err(e) => {
            let e = format!("Error when opening {file_name}: {e}");
            send_msg(MsgType::Error(e));
            return;
        }
    };

    let total_size = request.content_length();
    send_msg(MsgType::Starting { total_size });

    let mut stream = request.bytes_stream();

    while let Some(Ok(item)) = stream.next().await {
        match file.write(&item).await {
            Ok(bytes) => send_msg(MsgType::Written {
                bytes: bytes as u64,
            }),
            Err(e) => {
                let e = format!("failed do write to file {}: {e} Aborting", file_name);
                send_msg(MsgType::Error(e));
                return;
            }
        }
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

#[allow(unused)]
#[derive(Debug)]
enum MsgType {
    Error(String),
    Starting { total_size: Option<u64> },
    Written { bytes: u64 },
    Done,
}

#[derive(Debug, Parser)]
#[command(
    about = "\n\nUnofficial CLI for Binance public data\nMore information can be found on https://github.com/binance/binance-public-data/"
)]
struct Input {
    /// The ticker symbol you want to fetch data for.
    ticker: Ticker,
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

impl Input {
    fn to_fileinfo_iter(self) -> FileInfoIterator {
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
