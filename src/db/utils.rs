use time::OffsetDateTime;

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
