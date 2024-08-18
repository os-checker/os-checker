use color_eyre::Result;
use duct::cmd;

fn main() -> Result<()> {
    let git_time = cmd!("git", "log", "-1", "--format=%ci").read()?;
    println!("cargo::rustc-env=OS_CHECKER_GIT_TIME={git_time}");

    let git_sha = cmd!("git", "log", "-1", "--format=%H").read()?;
    println!("cargo::rustc-env=OS_CHECKER_GIT_SHA={git_sha}");

    Ok(())
}
