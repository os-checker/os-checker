use super::CargoMessage;

pub fn parse_lockbud_result(stderr: &[u8]) -> String {
    let tag = "[2025"; // 目前只能通过日志识别
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

#[cfg(test)]
mod tests {
    use crate::Result;

    fn get_lockbud_result() -> Result<String> {
        let toolchain = crate::utils::PLUS_TOOLCHAIN_LOCKBUD;
        let out = duct::cmd!("cargo", toolchain, "lockbud", "-k", "all")
            .dir("repos/os-checker-test-suite")
            .stderr_capture()
            .run()?;
        Ok(super::parse_lockbud_result(&out.stderr))
    }

    #[test]
    fn lockbud_output() -> Result<()> {
        let s = get_lockbud_result()?;
        println!("{s}");
        Ok(())
    }
}
