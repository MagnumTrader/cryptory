use std::collections::VecDeque;

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

#[derive(Debug)]
pub struct Tickerator {
    origin: VecDeque<Ticker>,
}

impl Iterator for Tickerator {
    type Item = Ticker;

    fn next(&mut self) -> Option<Self::Item> {
        self.origin.pop_front()
    }
}

impl<T> From<T> for Tickerator
where
    T: Into<VecDeque<Ticker>>,
{
    fn from(value: T) -> Self {
        Tickerator {
            origin: value.into(),
        }
    }
}
