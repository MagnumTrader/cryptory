mod fileinfo;
mod period;
mod timeframe;

use tokio::io::AsyncWriteExt;
use futures_util::StreamExt;

pub use fileinfo::{FileInfo, FileInfoIterator};
pub use period::Period;
pub use timeframe::TimeFrame;

use chrono::NaiveDate;
use tokio::sync::mpsc;

use std::fmt::Display;

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

pub async fn download_file(
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

pub struct Msg {
    pub file_id: usize,
    pub msg_type: MsgType,
}

impl Msg {
    fn new(file_id: usize, msg_type: MsgType) -> Self {
        Self { file_id, msg_type }
    }
}

#[derive(Debug)]
pub enum MsgType {
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

trait DateHelper: Sized {
    fn add_date_from_period(&self, period: &Period) -> Option<Self>;
    fn date_url_str(&self, period: &Period) -> FormattedDate;
}

impl DateHelper for NaiveDate {
    fn add_date_from_period(&self, period: &Period) -> Option<NaiveDate> {
        match period {
            Period::Daily { .. } => self.checked_add_days(chrono::Days::new(1)),
            Period::Monthly { .. } => self.checked_add_months(chrono::Months::new(1)),
        }
    }

    fn date_url_str(&self, period: &Period) -> FormattedDate {
        match period {
            Period::Daily { .. } => FormattedDate(self.to_string()),
            Period::Monthly { .. } => FormattedDate(self.format("%Y-%m").to_string()),
        }
    }
}

pub(super) struct FormattedDate(String);

impl Display for FormattedDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
