use crate::{db::InfoKey, prelude::*};

#[derive(Encode, Decode)]
pub struct CheckValue {
    pub keys: Vec<Keys>,
    /// The unix timestmap in milliseconds.
    pub timestamp_start: u64,
    /// The default is 0 and means all checks are not finished.
    /// The value should be updated once checks are done.
    pub timestamp_end: u64,
}

impl fmt::Debug for CheckValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CheckValue")
            .field("keys.len", &self.keys.len())
            .field(
                "timestamp_start",
                &parse_unix_timestamp_milli(self.timestamp_start),
            )
            .field(
                "timestamp_end",
                &parse_unix_timestamp_milli(self.timestamp_end),
            )
            .finish()
    }
}

impl Default for CheckValue {
    fn default() -> Self {
        Self {
            keys: vec![],
            timestamp_start: now(),
            timestamp_end: 0,
        }
    }
}

impl CheckValue {
    pub fn set_complete(&mut self) {
        self.timestamp_end = now();
        // sort keys by repo, but not include configs because normally each check item
        // should be with single config
        self.keys
            .sort_unstable_by(|a, b| a.info.repo.cmp(&b.info.repo));
    }

    pub fn is_complete(&self) -> bool {
        self.timestamp_end == 0
    }

    pub fn is_same_keys(&self, other: &Self) -> bool {
        // We could use PartialEq to do the comparison, but for simplicity, compare bytes here.
        musli::storage::to_vec(&self.keys).unwrap() == musli::storage::to_vec(&other.keys).unwrap()
    }

    /// Should be called once a new repo is being checked.
    pub fn push_info_key(&mut self, info: InfoKey) {
        self.keys.push(Keys { info });
    }
}

redb_value!(CheckValue, name: "OsCheckerCheckValue",
    read_err: "Not a valid check value.",
    write_err: "Check value can't be encoded to bytes.");

#[derive(Encode, Decode)]
pub struct Keys {
    pub info: InfoKey,
}

impl fmt::Debug for Keys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [user, repo] = &self.info.repo.user_repo();
        f.debug_struct("Keys")
            .field("info.repo", &format!("{user}/{repo}"))
            .finish()
    }
}
