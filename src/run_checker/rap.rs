use crate::config::Resolve;

pub fn rap_output(stderr: &[u8], stdout: &[u8], resolve: &Resolve) -> String {
    let mut output = parse_rap_result(stderr, stdout);
    if !output.is_empty() {
        output.insert_str(0, &resolve.display());
    }
    output
}

/// See https://github.com/Artisan-Lab/RAPx/issues/53
fn parse_rap_result(stderr: &[u8], stdout: &[u8]) -> String {
    // rap provides no-color option, but in case it doesn't work
    let stderr = String::from_utf8(strip_ansi_escapes::strip(stderr)).unwrap();
    if !stderr.contains("RAP|WARN") {
        return String::new();
    }
    String::from_utf8(strip_ansi_escapes::strip(stdout)).unwrap()
}

#[test]
pub fn get_rap_result() -> crate::Result<()> {
    let toolchain = crate::utils::PLUS_TOOLCHAIN_RAP;
    let out = duct::cmd!("cargo", toolchain, "rapx", "-F")
        .dir("../os-checker-test-suite/rap-checks-this")
        .stderr_capture()
        .unchecked()
        .run()?;
    println!(
        "stderr={}\nparsed={}",
        std::str::from_utf8(&out.stderr).unwrap(),
        parse_rap_result(&out.stderr, &out.stdout)
    );
    Ok(())
}
