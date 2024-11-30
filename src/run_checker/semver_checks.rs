pub fn parse(out: &std::process::Output, resolve: &crate::config::Resolve) -> String {
    // handle exit code 2 which is defined in Resolve cmd
    if !out.status.success() {
        let stdout = String::from_utf8(strip_ansi_escapes::strip(&out.stdout))
            .unwrap_or_else(|_| "Stdout contains non UTF8 chars.".to_owned());
        let stderr = String::from_utf8(strip_ansi_escapes::strip(&out.stderr))
            .unwrap_or_else(|_| "Stderr contains non UTF8 chars.".to_owned());

        return format!(
            "{}\nstdout:\n{stdout}\nstderr:\n{stderr}",
            resolve.display(),
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
