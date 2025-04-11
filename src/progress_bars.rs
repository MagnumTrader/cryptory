#![allow(unused, unreachable_code)]
use std::{borrow::Cow, collections::HashMap};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub struct ProgressBars {
    current_bars: HashMap<usize, (String, ProgressBar)>,
    style: ProgressStyle,
    multi_progress: MultiProgress,
}

impl ProgressBars {
    pub fn new() -> Self {
        let style =
            ProgressStyle::with_template("{msg} {bar:30} {decimal_bytes:>7}/{decimal_total_bytes}")
                .unwrap()
                .progress_chars("##-");

        Self {
            current_bars: HashMap::default(),
            multi_progress: MultiProgress::default(),
            style,
        }
    }

    pub fn new_bar(&mut self, bar_id: usize, name: String, total_size: Option<u64>) {
        if self.current_bars.contains_key(&bar_id) {
            panic!("this file id has already started a bar")
        }

        let pb = if let Some(total_size) = total_size {
            ProgressBar::new(total_size)
                .with_style(self.style.clone())
                .with_message(name.to_string())
        } else {
            let spinner = ProgressBar::new_spinner()
                .with_style(
                    ProgressStyle::with_template("{msg} {spinner} {decimal_bytes:>7}/{decimal_total_bytes}")
                        .expect("expect correct template"),
                )
                .with_message(name.to_string());
            spinner.enable_steady_tick(std::time::Duration::from_millis(200));
            spinner
        };

        let pb = self.multi_progress.add(pb);
        self.current_bars.insert(bar_id, (name, pb));
    }

    pub fn increment(&mut self, bar_id: usize, with: u64) {
        let bar = self
            .get_mut_bar(bar_id)
            .expect("Internal error: expected to find progressbar");

        bar.inc(with);

        if bar.should_finish() {
            self.finish(bar_id, Some("Done"));
        }
    }

    pub fn finish(&mut self, bar_id: usize, msg: Option<impl ToString>) {
        let bar = self
            .get_mut_bar(bar_id)
            .expect("Internal error: expected to find progressbar");
        match msg {
            Some(msg) => bar.finish_with_message(msg.to_string()),
            None => bar.finish(),
        }
    }
    /// Abondon a bar with an optional message.
    /// Does nothing if bar is not found
    pub fn abandon(&mut self, bar_id: usize, msg: Option<impl ToString>) {
        let Ok(bar) = self.get_mut_bar(bar_id) else {
            return;
        };

        match msg {
            Some(msg) => bar.abandon_with_message(msg.to_string()),
            None => bar.abandon(),
        }
    }

    fn get_mut_bar(&mut self, bar_id: usize) -> Result<&mut ProgressBar, ProgressBarsError> {
        let bar = &mut self
            .current_bars
            .get_mut(&bar_id)
            .ok_or(ProgressBarsError::IdDoesntExist)?
            .1;
        Ok(bar)
    }
}

trait ProgressBarHelper {
    fn should_finish(&self) -> bool;
}
impl ProgressBarHelper for ProgressBar {
    fn should_finish(&self) -> bool {
        self.position() >= self.length().unwrap_or(u64::MAX)
    }
}

#[derive(Debug)]
pub enum ProgressBarsError {
    IdDoesntExist,
}
impl std::fmt::Display for ProgressBarsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
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
