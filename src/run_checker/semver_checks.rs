pub fn parse(out: &std::process::Output, resolve: &crate::config::Resolve) -> String {
    // handle exit code 2 which is defined in Resolve cmd
    if !out.status.success() {
        // already no color
        return format!(
            "{}{}",
            resolve.display(),
            String::from_utf8_lossy(&out.stdout)
        );
    }
    String::new()
}

#[test]
fn semver_checks_output() -> crate::Result<()> {
    let output = duct::cmd!("cargo", "semver-checks", "--color=never")
        .dir("os-checker-types")
        .read()?;
    println!("{output}");
    Ok(())
}
