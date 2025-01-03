pub use camino::{Utf8Path, Utf8PathBuf};
pub use compact_str::CompactString as XString;
pub use eyre::Result;
pub use indexmap::IndexMap;
pub use musli::{Decode, Encode};
pub use serde::{Deserialize, Serialize};
pub use std::fmt;

use std::{fmt::Debug, hash::Hash};
use time::OffsetDateTime;

/// Returns the current unix timestamp in milliseconds.
pub fn now() -> u64 {
    let t = time::OffsetDateTime::from(std::time::SystemTime::now());
    unix_timestamp_milli(t)
}

pub fn unix_timestamp_milli(t: OffsetDateTime) -> u64 {
    let milli = t.millisecond() as u64;
    let unix_t_secs = t.unix_timestamp() as u64;
    unix_t_secs * 1000 + milli
}

pub fn parse_unix_timestamp_milli(ts: u64) -> OffsetDateTime {
    let t_with_millis = ts / 1000;
    match OffsetDateTime::from_unix_timestamp((t_with_millis) as i64) {
        Ok(t) => t
            .replace_millisecond((ts - t_with_millis * 1000) as u16)
            .unwrap()
            .to_offset(time::UtcOffset::from_hms(8, 0, 0).unwrap()),
        Err(err) => panic!("{ts} 无法转回时间：{err}"),
    }
}

#[macro_export]
macro_rules! redb_value {
    (
      $t:ident, name: $name:literal,
      read_err: $read_err:literal, write_err: $write_err:literal
    ) => {
        impl ::redb::Value for $t {
            type SelfType<'a>
                = Self
            where
                Self: 'a;

            type AsBytes<'a>
                = Vec<u8>
            where
                Self: 'a;

            fn fixed_width() -> Option<usize> {
                None
            }

            fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
            where
                Self: 'a,
            {
                use std::error::Error;
                match ::musli::storage::from_slice(data) {
                    Ok(res) => res,
                    Err(err) => {
                        panic!(
                            "{}\nerr (debug) = {err:?}\nerr (display) = {err}\nerr (source) = {:?}",
                            $read_err, err.source()
                        )
                    }
                }
            }

            fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
            where
                Self: 'a,
                Self: 'b,
            {
                ::musli::storage::to_vec(value).expect($write_err)
            }

            fn type_name() -> redb::TypeName {
                ::redb::TypeName::new($name)
            }
        }
    };
    (@key
      $t:ident, name: $name:literal,
      read_err: $read_err:literal, write_err: $write_err:literal
    ) => {
        redb_value!($t, name: $name, read_err: $read_err, write_err: $write_err);
        impl ::redb::Key for $t {
            fn compare(data1: &[u8], data2: &[u8]) -> ::std::cmp::Ordering {
                data1.cmp(data2)
            }
        }
    };
}

fn count_key<K: Hash + Eq + Debug>(k: K, map: &mut IndexMap<K, u8>) {
    if let Some(count) = map.get_mut(&k) {
        error!(key = ?k, "The occurrence shouldn't be more than 1.");
        *count += 1;
    } else {
        map.insert(k, 1);
    }
}

pub fn check_key_uniqueness<K: Hash + Eq + Debug>(
    iter: impl ExactSizeIterator<Item = K>,
) -> Result<()> {
    let mut count = IndexMap::with_capacity(iter.len());
    iter.for_each(|k| count_key(k, &mut count));
    let invalid: Vec<_> = count.iter().filter(|(_, c)| **c != 1u8).collect();
    ensure!(invalid.is_empty(), "invalid = {invalid:#?}");
    Ok(())
}
