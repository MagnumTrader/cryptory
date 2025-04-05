use std::{fmt::Display, str::FromStr};

/// Valid representation of timeframes that can be fetched from binance.
#[derive(Debug, Clone)]
pub struct TimeFrame(String);

impl FromStr for TimeFrame {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "12h" | "15m" | "1d" | "1h" | "1m" | "1mo" | "1s" | "1w" | "2h" | "30m" | "3d"
            | "3m" | "4h" | "5m" | "6h" | "8h" => Ok(TimeFrame(s.to_string())),
            _ => Err("Invalid timeframe! Valid values are:\n 1s 1m 3m 5m 15m 30m 1h 2h 4h 6h 8h 12h 1d 1w 1mo")

        }
    }
}

impl Display for TimeFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
