use std::io::Write;

/// See https://github.com/Artisan-Lab/RAP/issues/53
pub fn parse_rap_result(stderr: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr);
    let mut writer = strip_ansi_escapes::Writer::new(Vec::with_capacity(stderr.len() / 2));

    for line in stderr.lines() {
        if line.contains("RAP-FRONT|WARN") {
            if let Err(err) = writer.write_all(line.as_bytes()) {
                error!(line, ?err, "strip_ansi_escapes for rap output");
            }
        }
    }
    _ = writer.flush();
    let bytes = writer.into_inner().unwrap();
    String::from_utf8_lossy(&bytes).into_owned()
}
