use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, TimeZone, Utc};
use lazy_static::lazy_static;
use serde::export::Formatter;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(PartialOrd, Ord, PartialEq, Eq, Hash, Clone, Copy)]
pub struct Ulid {
    inner: u128,
}

pub struct UlidGenerator {
    previous: Ulid,
}

const ULID_LEN: usize = 26;
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
macro_rules! bitmask {
    ($len:expr) => {
        ((1 << $len) - 1)
    };
}
lazy_static! {
    static ref MAX_RANDOM: u128 = bitmask!(Ulid::RAND_BITS);
}
lazy_static! {
    static ref LOOKUP: [Option<u8>; 256] = {
        let mut lookup = [None; 256];
        for (i, &c) in ALPHABET.iter().enumerate() {
            lookup[c as usize] = Some(i as u8);
            if !(c as char).is_numeric() {
                //lowercase
                lookup[(c+32) as usize] = Some(i as u8);
            }
        }
        lookup
    };
}

/// Error while trying to generate a monotonic increment in the same millisecond
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum MonotonicError {
    /// Would overflow into the next millisecond
    Overflow,
}

fn encode(mut value: u128) -> String {
    let mut buffer: [u8; ULID_LEN] = [ALPHABET[0]; ULID_LEN];

    for i in 0..ULID_LEN {
        buffer[25 - i] = ALPHABET[(value & 0x1f) as usize];
        value >>= 5;
    }

    String::from_utf8(buffer.to_vec()).expect("unexpected failure in base32 encode for ulid")
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum DecodeError {
    InvalidLength,
    InvalidChar,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let text = match *self {
            DecodeError::InvalidLength => "invalid length",
            DecodeError::InvalidChar => "invalid character",
        };
        write!(f, "{}", text)
    }
}

impl std::error::Error for DecodeError {}

pub fn decode(encoded: &str) -> Result<u128, DecodeError> {
    if encoded.len() != ULID_LEN {
        return Err(DecodeError::InvalidLength);
    }
    let mut value: u128 = 0;
    let bytes = encoded.as_bytes();

    for i in 0..ULID_LEN {
        if let Some(val) = LOOKUP[bytes[i] as usize] {
            value = (value << 5) | u128::from(val);
        } else {
            return Err(DecodeError::InvalidChar);
        }
    }

    Ok(value)
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
        (u128::from(msb) << 64 | u128::from(lsb)).into()
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

    pub fn from_string(input: &str) -> Result<Ulid, DecodeError> {
        decode(input).map(|i| i.into())
    }

    pub fn from_u128(inner: u128) -> Ulid {
        Ulid { inner }
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

impl fmt::Debug for Ulid {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.to_string())
    }
}

impl fmt::Display for Ulid {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", encode(self.inner))
    }
}

impl FromStr for Ulid {
    type Err = DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ulid::from_string(s)
    }
}

impl std::convert::From<u128> for Ulid {
    fn from(u: u128) -> Self {
        Ulid::from_u128(u)
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

impl<'de> Deserialize<'de> for Ulid {
    fn deserialize<D>(deserializer: D) -> Result<Ulid, D::Error>
    where
        D: Deserializer<'de>,
    {
        // todo: inplace to reduce garbage?
        let deserialized_str = String::deserialize(deserializer)?;
        Self::from_string(&deserialized_str).map_err(serde::de::Error::custom)
    }
}

impl Default for UlidGenerator {
    fn default() -> Self {
        UlidGenerator {
            previous: Ulid::default(),
        }
    }
}

impl UlidGenerator {
    pub fn new() -> UlidGenerator {
        UlidGenerator::default()
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

#[allow(unused_variables)]
#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use fehler::*;

    use crate::events::ulid::{Ulid, UlidGenerator};

    #[test]
    #[throws(anyhow::Error)]
    fn test_smoke() {
        let ulid = UlidGenerator::new().generate()?;
        assert_eq!(ulid, Ulid::from_str(&ulid.to_string())?);
    }
}
