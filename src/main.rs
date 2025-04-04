mod fetch;
use fetch::{FileInfo, FileInfoIterator, Period, TimeFrame};

use clap::Parser;
use futures_util::StreamExt;
use tokio::{io::AsyncWriteExt, sync::mpsc};

use std::{fmt::Display, str::FromStr};

// TODO: Move async task to function instead
// TODO: Use file ids instead
// TODO: track progress of files
// TODO: Implement progress bars

#[tokio::main]
async fn main() {
    let input = Input::parse();
    let client = reqwest::Client::new();

    let (tx, mut rx) = mpsc::unbounded_channel::<Msg>();

    for (fileinfo, file_id) in input.to_fileinfo_iter().zip(1..) {
        tokio::spawn(download_file(
            fileinfo,
            file_id,
            client.clone(),
            tx.clone(),
            input.overwrite,
        ));
    }

    // Dropping tx to make sure main thread doesn't deadlock
    // waiting for this handle to send or drop
    drop(tx);

    while let Some(msg) = rx.recv().await {
        // TODO: Handle msg types here
        println!("{} is {:?}", msg.file_id, msg.msg_type)
    }
}

async fn download_file(
    fileinfo: FileInfo,
    file_id: usize,
    local_client: reqwest::Client,
    local_tx: mpsc::UnboundedSender<Msg>,
    overwrite: bool,
) {
    let send_msg = move |msg: MsgType| {
        let _ = local_tx.send(Msg::new(file_id, msg));
    };

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

    let mut file = match open_file(file_path.clone(), owerwrite).await {
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
            Ok(bytes) => send_msg(MsgType::Written { bytes }),
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
    Written { bytes: usize },
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
    overwrite: bool,
) -> Result<tokio::fs::File, std::io::Error> {
    let mut open_options = tokio::fs::OpenOptions::new();

    if !input.overwrite {
        // create is ignored when create new is set.
        // so we can always include it.
        open_options.create_new(true);
    }
    open_options.create(true).write(true).open(path).await
}
