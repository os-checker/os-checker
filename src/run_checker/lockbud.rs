use super::CargoMessage;

#[cfg(test)]
pub fn get_lockbud_result() -> crate::Result<String> {
    let out = duct::cmd!("cargo", "+nightly-2024-05-21", "lockbud", "-k", "all")
        .dir("repos/os-checker-test-suite")
        .stderr_capture()
        .run()?;
    Ok(parse_lockbud_result(&out.stderr))
}

pub fn parse_lockbud_result(stderr: &[u8]) -> String {
    let tag = "[2024"; // 目前只能通过日志识别
    let mut count = 0usize;
    let mut v = Vec::new();
    for mes in CargoMessage::parse_stream(stderr).flatten() {
        if let cargo_metadata::Message::TextLine(line) = mes {
            if line.starts_with(tag) {
                count += 1;
            }
            if count != 0 {
                v.push(line);
            }
            if count == 2 {
                count = 0;
            }
        }
    }
    v.join("\n")
}
