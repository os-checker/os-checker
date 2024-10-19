use std::fmt::Write;

/// See https://github.com/Artisan-Lab/RAP/issues/53
pub fn parse_rap_result(stderr: &[u8]) -> String {
    // rap doesn't provide no-color option
    let stderr = String::from_utf8(strip_ansi_escapes::strip(stderr)).unwrap();
    let mut rap_output = String::with_capacity(stderr.len() / 2);

    for line in stderr.lines() {
        if line.contains("RAP|WARN") {
            _ = writeln!(&mut rap_output, "{line}");
        }
    }
    info!(rap_output, ?stderr);
    rap_output
}

#[test]
pub fn get_rap_result() -> crate::Result<()> {
    let toolchain = crate::utils::PLUS_TOOLCHAIN_RAP;
    let out = duct::cmd!("cargo", toolchain, "rap", "-F")
        .dir("../os-checker-test-suite/rap-checks-this")
        .stderr_capture()
        .unchecked()
        .run()?;
    println!(
        "stderr={}\nparsed={}",
        std::str::from_utf8(&out.stderr).unwrap(),
        parse_rap_result(&out.stderr)
    );
    Ok(())
}
