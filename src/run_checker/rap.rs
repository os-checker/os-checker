use std::io::Write;

/// See https://github.com/Artisan-Lab/RAP/issues/53
pub fn parse_rap_result(stderr: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr);
    let mut writer = strip_ansi_escapes::Writer::new(Vec::with_capacity(stderr.len() / 2));

    for line in stderr.lines() {
        if line.contains("RAP-FRONT|WARN") {
            match writer.write_all(line.as_bytes()) {
                Ok(_) => _ = writer.write(b"\n"),
                Err(err) => error!(line, ?err, "strip_ansi_escapes for rap output"),
            }
        }
    }
    _ = writer.flush();
    let bytes = writer.into_inner().unwrap();
    let rap_output = String::from_utf8_lossy(&bytes).into_owned();
    info!(rap_output);
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
