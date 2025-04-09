#[derive(Debug, Clone)]
pub struct Ticker(String);

impl std::fmt::Display for Ticker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for Ticker {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // validate symbols here i guess url will fail?
        Ok(Self(s.to_uppercase()))
    }
}

pub struct Tickerator<'a> {
    origin: &'a [Ticker],
    current: usize,
}

impl<'a> Tickerator<'a> {
    pub fn reset(&mut self) {
        self.current = 0
    }
}

impl<'a> Iterator for Tickerator<'a> {
    type Item = &'a Ticker;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.origin.len() {
            return None;
        };

        let ticker_index = self.current;
        self.current += 1;

        Some(&self.origin[ticker_index])
    }
}

impl<'a> From<&'a [Ticker]> for Tickerator<'a> {
    fn from(value: &'a [Ticker]) -> Self {
        Tickerator {
            origin: value,
            current: 0,
        }
    }
}
