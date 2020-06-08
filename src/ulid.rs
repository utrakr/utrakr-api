use chrono::{DateTime, TimeZone, Utc};
use lazy_static::lazy_static;
use serde::export::fmt::Display;
use serde::export::Formatter;
use serde::{Serialize, Serializer};
use std::fmt;

#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Hash, Clone, Copy)]
pub struct Ulid {
    inner: u128,
}

pub struct UlidGenerator {
    previous: Ulid,
}

const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
macro_rules! bitmask {
    ($len:expr) => {
        ((1 << $len) - 1)
    };
}
lazy_static! {
    static ref MAX_RANDOM: u128 = bitmask!(Ulid::RAND_BITS);
}

/// Error while trying to generate a monotonic increment in the same millisecond
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum MonotonicError {
    /// Would overflow into the next millisecond
    Overflow,
}

fn encode(mut value: u128) -> String {
    let mut buffer: [u8; 26] = [ALPHABET[0]; 26];

    for i in 0..26 {
        buffer[25 - i] = ALPHABET[(value & 0x1f) as usize];
        value >>= 5;
    }

    String::from_utf8(buffer.to_vec()).expect("unexpected failure in base32 encode for ulid")
}

impl Ulid {
    const TIME_BITS: u8 = 48;
    const RAND_BITS: u8 = 80;

    pub fn from_datetime_with_source<T, R>(datetime: DateTime<T>, source: &mut R) -> Ulid
    where
        T: TimeZone,
        R: rand::Rng,
    {
        let timestamp = datetime.timestamp_millis();
        let timebits = (timestamp & bitmask!(Self::TIME_BITS)) as u64;

        let msb = timebits << 16 | u64::from(source.gen::<u16>());
        let lsb = source.gen::<u64>();
        Ulid {
            inner: u128::from(msb) << 64 | u128::from(lsb),
        }
    }

    /// Increment the random number, make sure that the ts millis stays the same
    fn increment(&self) -> Option<Ulid> {
        if (self.inner & *MAX_RANDOM) == *MAX_RANDOM {
            None
        } else {
            Some(Ulid {
                inner: self.inner + 1,
            })
        }
    }

    pub fn to_string(&self) -> String {
        encode(self.inner)
    }

    pub fn timestamp_millis(&self) -> i64 {
        (self.inner >> Self::RAND_BITS) as i64
    }
}

impl Default for Ulid {
    fn default() -> Self {
        Ulid {
            inner: u128::default(),
        }
    }
}

impl Display for Ulid {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.to_string())
    }
}

impl fmt::Display for MonotonicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let text = match *self {
            MonotonicError::Overflow => "Ulid random bits would overflow",
        };
        write!(f, "{}", text)
    }
}

impl std::error::Error for MonotonicError {}

impl Serialize for Ulid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl UlidGenerator {
    pub fn new() -> UlidGenerator {
        UlidGenerator {
            previous: Ulid::default(),
        }
    }

    pub fn generate(&mut self) -> Result<Ulid, MonotonicError> {
        self.generate_from_datetime(Utc::now())
    }

    pub fn generate_from_datetime<T: TimeZone>(
        &mut self,
        datetime: DateTime<T>,
    ) -> Result<Ulid, MonotonicError> {
        self.generate_from_datetime_with_source(datetime, &mut rand::thread_rng())
    }

    pub fn generate_from_datetime_with_source<T, R>(
        &mut self,
        datetime: DateTime<T>,
        source: &mut R,
    ) -> Result<Ulid, MonotonicError>
    where
        T: TimeZone,
        R: rand::Rng,
    {
        let last_millis = self.previous.timestamp_millis();

        // increment instead of generating a new random so that it is monotonic
        if datetime.timestamp_millis() <= last_millis {
            if let Some(next) = self.previous.increment() {
                self.previous = next;
                Ok(next)
            } else {
                Err(MonotonicError::Overflow)
            }
        } else {
            let next = Ulid::from_datetime_with_source(datetime, source);
            self.previous = next;
            Ok(next)
        }
    }
}
