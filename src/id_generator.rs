use chrono::{DateTime, Duration, Utc};
use lazy_static::lazy_static;
use rand::{Rng, thread_rng};

#[derive(Clone)]
pub struct IdGenerator {
    len: u8,
}

#[allow(dead_code)] // reg alphabet for testing, to make it easier to understand
const ALPHABET: &[u8; 64] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-";
// mix up the alphabet so that its harder for humans to see any patterns
const CRAZY_ALPHABET: &[u8; 64] =
    b"pm1otTq7cI4J9Oy_Fd2aQezXVU5S3gYlrxnv6fPshHGRDbWjkuiLZ80wBNA-KCME";
// time constants
const TIME_BITS: u32 = 30;
lazy_static! {
    static ref MAX_SECONDS: u32 = {
        2_u32.pow(TIME_BITS)
    };

    // "2020-01-01" + (2^30 seconds) = Jan 2054 - seems like enough time
    static ref OUR_EPOCH: DateTime<Utc> = {
        "2020-01-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap()
    };

    static ref MAX_TIME: DateTime<Utc> = *OUR_EPOCH + Duration::seconds(*MAX_SECONDS as i64);
}

impl IdGenerator {
    pub fn new(len: u8) -> IdGenerator {
        if len <= 6 {
            panic!("too small")
        }
        if len > 20 {
            panic!("too large")
        }
        IdGenerator { len }
    }

    pub fn gen_id(&self) -> String {
        let now = Utc::now();
        if now >= *MAX_TIME {
            panic!("something has gone very wrong, we are in {}", *OUR_EPOCH);
        }

        let rn = thread_rng().gen::<u32>();
        let ts = now.signed_duration_since(*OUR_EPOCH).num_seconds() as u32;
        id_from_ts_and_random(CRAZY_ALPHABET, self.len as usize, ts, rn)
    }
}

fn id_from_ts_and_random(alpha: &[u8; 64], len: usize, ts: u32, rn: u32) -> String {
    assert!(ts < *MAX_SECONDS);
    let mut buf = Vec::with_capacity(len);

    let mut ts_value = ts;
    let mut rn_value = rn;

    const TIME_SHIFT: u32 = 5;
    assert_eq!(TIME_BITS % TIME_SHIFT, 0);
    let num_time_chars = (TIME_BITS / TIME_SHIFT) as usize;

    for i in 0..len {
        let v = if i < num_time_chars {
            let r = ((ts_value & 0x1f) << 1) | (rn_value & 0x01);
            ts_value >>= TIME_SHIFT;
            rn_value >>= 6 - TIME_SHIFT;
            r
        } else {
            let r = rn_value & 0x3f;
            rn_value >>= 6;
            r
        };
        buf.push(alpha[v as usize]);
    }

    buf.reverse();
    String::from_utf8(buf).expect("unexpected failure in encode for id gen")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple() {
        let s1 = IdGenerator::new(7).gen_id();
        println!("{}", s1);
        assert_eq!(s1.len(), 7);
        let s2 = IdGenerator::new(8).gen_id();
        println!("{}", s2);
        assert_eq!(s2.len(), 8);
    }

    #[test]
    fn exact() {
        let now = "2020-05-23T15:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let dur = now.signed_duration_since(*OUR_EPOCH).num_seconds() as u32;

        let a = ALPHABET;
        assert_eq!(id_from_ts_and_random(a, 7, 0, 1), "aaaaaab");
        assert_eq!(id_from_ts_and_random(a, 7, 0, 2), "aaaaaba");
        assert_eq!(id_from_ts_and_random(a, 7, 0, 3), "aaaaabb");
        assert_eq!(id_from_ts_and_random(a, 7, 0, 2_u32.pow(11)), "Gaaaaaa");
        assert_eq!(id_from_ts_and_random(a, 7, 0, 2_u32.pow(12)), "aaaaaaa");
        assert_eq!(id_from_ts_and_random(a, 8, 0, 2_u32.pow(12)), "baaaaaaa");
        assert_eq!(id_from_ts_and_random(a, 7, 1, 0), "aaaaaac");
        assert_eq!(id_from_ts_and_random(a, 7, *MAX_SECONDS - 1, 0), "a______");

        // test some rnd
        assert_eq!(id_from_ts_and_random(a, 7, dur, 2_u32.pow(6)), "baw0SwG");
        assert_eq!(id_from_ts_and_random(a, 7, dur, 2_u32.pow(7)), "caw0SwG");
        assert_eq!(id_from_ts_and_random(a, 7, dur, 2_u32.pow(11)), "Gaw0SwG");

        // overflow for 7
        assert_eq!(id_from_ts_and_random(a, 7, dur, 2_u32.pow(12)), "aaw0SwG");
        // works for 8
        assert_eq!(id_from_ts_and_random(a, 8, dur, 2_u32.pow(12)), "baaw0SwG");
    }

    #[test]
    #[should_panic(expected = "too small")]
    fn too_small() {
        IdGenerator::new(6);
    }
}
