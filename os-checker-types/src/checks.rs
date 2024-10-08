use crate::{
    db::{CacheRepoKey, InfoKey},
    prelude::*,
};

#[derive(Encode, Decode, Debug)]
pub struct CheckValue {
    pub keys: Vec<Keys>,
    /// The unix timestmap in milliseconds.
    pub timestamp_start: u64,
    /// The default is 0 and means all checks are not finished.
    /// The value should be updated once checks are done.
    pub timestamp_end: u64,
}

redb_value!(CheckValue, name: "OsCheckerCheckValue",
    read_err: "Not a valid check value.",
    write_err: "Check value can't be encoded to bytes.");

#[derive(Encode, Decode, Debug)]
pub struct Keys {
    pub cache: Vec<CacheRepoKey>,
    pub info: InfoKey,
}
