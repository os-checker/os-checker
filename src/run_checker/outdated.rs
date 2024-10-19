pub fn parse_outdated(out: &std::process::Output) -> String {
    // handle exit code 2 which is defined in Resolve cmd
    if out.status.code() == Some(2) {
        // already no color
        return String::from_utf8_lossy(&out.stdout).into_owned();
    }
    String::new()
}
