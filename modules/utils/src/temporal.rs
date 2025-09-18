use soroban_sdk::contracttype;

#[contracttype]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Delay(u64);

impl Delay {
    pub const fn from_seconds(seconds: u64) -> Self {
        Self(seconds)
    }

    pub const fn as_seconds(self) -> u64 {
        self.0
    }

    pub fn from_timestamp_diff(now: u64, past_timestamp: u64) -> Option<Self> {
        now.checked_sub(past_timestamp).map(Self)
    }

    pub fn from_timestamp_diff_expect(now: u64, past_timestamp: u64, msg: &str) -> Self {
        Self::from_timestamp_diff_with_tolerance(now, past_timestamp, 60, msg)
    }

    pub fn from_timestamp_diff_with_tolerance(now: u64, past_timestamp: u64, tolerance_seconds: u64, msg: &str) -> Self {
        if past_timestamp <= now {
            // Normal case: past timestamp is in the past
            Self(now - past_timestamp)
        } else if past_timestamp - now <= tolerance_seconds {
            // Oracle timestamp is slightly in the future but within tolerance
            // Treat as zero delay to handle clock drift gracefully
            Self(0)
        } else {
            // Oracle timestamp is too far in the future, this indicates a real problem
            panic!("{}", msg)
        }
    }

    pub const ZERO: Self = Self(0);
}

impl PartialEq<u64> for Delay {
    fn eq(&self, other: &u64) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<u64> for Delay {
    fn partial_cmp(&self, other: &u64) -> Option<core::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl From<u64> for Delay {
    fn from(seconds: u64) -> Self {
        Self::from_seconds(seconds)
    }
}

impl From<Delay> for u64 {
    fn from(delay: Delay) -> Self {
        delay.as_seconds()
    }
}
