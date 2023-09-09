#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Deserialize, serde::Serialize,
)]
pub struct Duration(std::time::Duration);

impl std::fmt::Display for Duration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // we use the debug format of the inner duration, which is good enough
        write!(f, "{:?}", self.0)
    }
}

impl From<std::time::Duration> for Duration {
    fn from(value: std::time::Duration) -> Self {
        Self(value)
    }
}

impl From<Duration> for std::time::Duration {
    fn from(value: Duration) -> Self {
        value.0
    }
}

impl Duration {
    pub const ZERO: Self = Self(std::time::Duration::ZERO);
    pub const FOREVER: Self = Self(std::time::Duration::MAX);

    pub const QUANTUM: Self = Self::of(100, DurationUnit::Milliseconds);
    pub const STOP_TIMEOUT: Self = Self::of(10, DurationUnit::Seconds);

    pub const fn of(magnitude: u64, unit: DurationUnit) -> Self {
        Self(match unit {
            DurationUnit::Milliseconds => std::time::Duration::from_millis(magnitude),
            DurationUnit::Seconds => std::time::Duration::from_secs(magnitude),
        })
    }

    pub fn sleep(&self) {
        std::thread::sleep((*self).into())
    }
}

pub enum DurationUnit {
    Milliseconds,
    Seconds,
}
