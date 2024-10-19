pub fn parse_outdated(out: &std::process::Output, resolve: &crate::config::Resolve) -> String {
    // handle exit code 2 which is defined in Resolve cmd
    if out.status.code() == Some(2) {
        // already no color
        return format!(
            "{}{}",
            resolve.display(),
            String::from_utf8_lossy(&out.stdout)
        );
    }
    String::new()
}
