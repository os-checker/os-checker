use crate::prelude::*;

#[derive(Debug, Encode, Decode, Clone)]
pub struct InfoKey {
    pub repo: crate::db::CacheRepo,
    #[musli(with = musli::serde)]
    pub config: crate::db::RepoConfig,
}

impl InfoKey {
    pub fn user_repo(&self) -> [&str; 2] {
        self.repo.user_repo()
    }
}

redb_value!(@key InfoKey, name: "OsCheckerInfoKey",
    read_err: "Not a valid info key.",
    write_err: "Info key can't be encoded to bytes."
);

#[derive(Debug, Encode, Decode)]
pub struct Info {
    /// 该仓库的检查是否全部完成
    pub complete: bool,
    /// 缓存信息
    pub caches: Vec<crate::db::CacheRepoKey>,
    /// 仓库最新提交信息
    pub latest_commit: LatestCommit,
}

redb_value!(Info, name: "OsCheckerInfo",
    read_err: "Not a valid info value.",
    write_err: "Info value can't be encoded to bytes."
);

#[derive(Debug, Encode, Decode)]
pub struct LatestCommit {
    pub sha: String,
    pub mes: String,
    pub author: Committer,
    pub committer: Committer,
}

#[derive(Encode, Decode)]
pub struct Committer {
    // store as unix timestemp milli
    pub datetime: u64,
    #[musli(with = musli::serde)]
    pub email: String,
    #[musli(with = musli::serde)]
    pub name: XString,
}

impl fmt::Debug for Committer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Committer")
            .field(
                "datetime",
                &super::parse_unix_timestamp_milli(self.datetime),
            )
            .field("email", &self.email)
            .field("name", &self.name)
            .finish()
    }
}

// fn deserialize_date<'de, D>(deserializer: D) -> Result<u64, D::Error>
// where
//     D: serde::Deserializer<'de>,
// {
//     let dt = <&str>::deserialize(deserializer)?;
//     Ok(parse_datetime(dt))
// }
//
// fn parse_datetime(dt: &str) -> u64 {
//     const DESC: &[time::format_description::BorrowedFormatItem<'static>] =
//         time::macros::format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]Z");
//     let utc = time::PrimitiveDateTime::parse(dt, DESC)
//         .unwrap()
//         .assume_utc();
//     let local = utc.to_offset(time::UtcOffset::from_hms(8, 0, 0).unwrap());
//     unix_timestamp_milli(local)
// }
