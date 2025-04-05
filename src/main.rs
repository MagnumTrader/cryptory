mod fetch;
use fetch::{FileInfo, FileInfoIterator, Period, TimeFrame};

use clap::Parser;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::{io::AsyncWriteExt, sync::mpsc};

use std::{collections::HashMap, fmt::Display, str::FromStr};

// TODO: Move async task to function instead
// TODO: track progress of files
// TODO: Implement progress bars

#[tokio::main]
async fn main() {
    let input = Input::parse();
    let client = reqwest::Client::new();

    let (tx, mut rx) = mpsc::unbounded_channel::<Msg>();
    let overwrite = input.overwrite;

    for fileinfo in input.to_fileinfo_iter() {
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

    let mut bars: HashMap<usize, ProgressBar> = HashMap::new();
    let mut errors = Vec::new();

    let style = ProgressStyle::with_template(
        "{bar:30.cyan/blue} {decimal_bytes:>7}/{decimal_total_bytes} {msg}",
    )
    .unwrap();
    let mpg = indicatif::MultiProgress::new();

    while let Some(msg) = rx.recv().await {
        let Msg { file_id, msg_type } = msg;
        match msg_type {
            MsgType::Written { bytes } => expect_file_id(&bars, file_id).inc(bytes),
            MsgType::Done => expect_file_id(&bars, file_id).finish_with_message("Done"),
            MsgType::Starting { total_size } => {
                let pb = if let Some(total_size) = total_size {
                    ProgressBar::new(total_size)
                        .with_style(style.clone().progress_chars(">>-"))
                        .with_message("")
                } else {
                    let spinner = ProgressBar::new_spinner();
                    spinner.enable_steady_tick(std::time::Duration::from_millis(200));
                    spinner
                };
                let pb = mpg.add(pb);
                bars.insert(file_id, pb);
            }
            MsgType::Error(e) => {
                errors.push(e);
                expect_file_id(&bars, file_id).abandon_with_message("ERROR");
            }
        }
    }
}

fn expect_file_id(bars: &HashMap<usize, ProgressBar>, file_id: usize) -> &ProgressBar {
    bars.get(&file_id).expect("expect bar to exist")
}

async fn download_file(
    fileinfo: FileInfo,
    local_client: reqwest::Client,
    local_tx: mpsc::UnboundedSender<Msg>,
    overwrite: bool,
) {
    let FileInfo {
        source_url,
        file_path,
        file_id,
    } = fileinfo;

    let send_msg = move |msg: MsgType| {
        let _ = local_tx.send(Msg::new(file_id, msg));
    };

    let file_name = file_path
        .file_stem()
        .expect("we expect a file stem")
        .to_str()
        .unwrap()
        .to_string();

    let request = match local_client.get(source_url).send().await {
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

    let mut file = match open_file(file_path.clone(), overwrite).await {
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
    // Use ID instead
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
