pub fn parse(out: &std::process::Output, resolve: &crate::config::Resolve) -> String {
    let output = String::from_utf8_lossy(&out.stdout);
    // cargo-geiger doesn't report via exitcode,
    // so we have to search by chars.
    // ! must appear once in the header description
    if output.matches("! ").nth(1).is_some() {
        format!("{}{output}", resolve.display())
    } else {
        String::new()
    }
}
