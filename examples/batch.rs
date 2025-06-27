// #!/usr/bin/env -S cargo +nightly -Zscript

#[macro_use]
extern crate tracing;

use argh::FromArgs;
use camino::Utf8PathBuf;
use color_eyre::Result;
use duct::cmd;
use std::env::var;

#[derive(FromArgs, Debug)]
/// Run os-checker in batch.
struct Batch {
    /// forward --size
    #[argh(option)]
    size: String,
    /// don't upload the cache.redb to githhub
    #[argh(switch)]
    no_upload: bool,
    /// args (i.e. `-- arg1 arg2 ...`) that will be passed to os-checker
    #[argh(positional)]
    os_checker_args: Vec<String>,
}

const DB: &str = "cache.redb";
const OS_CHECKER_CONFIGS: &str = "OS_CHECKER_CONFIGS";

fn main() -> Result<()> {
    logger::init();
    let batch: Batch = argh::from_env();
    info!(?batch);

    let base_dir = base_dir();
    std::env::set_current_dir(&base_dir)?;
    info!(%base_dir, "set_current_dir");
    let config_dir = base_dir.join("config");
    let batch_dir = base_dir.join("batch");

    let mut args = Vec::<&str>::with_capacity(16);
    args.push("batch");
    args.extend(["--out-dir", config_dir.as_str()]);
    args.extend(["--size", &batch.size]);

    let configs = var(OS_CHECKER_CONFIGS)?;
    info!(?configs);
    cmd("os-checker", args)
        .env(OS_CHECKER_CONFIGS, configs)
        .run()?;

    let [mut count_json_file, mut count_repos] = [0usize; 2];
    // NOTE: 这里没有对文件排序，所以不是完全按字母表顺序检查（虽然文件内的仓库是字母顺序）
    for entry in config_dir.read_dir_utf8()? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_file() && path.extension() == Some("json") {
            info!(batch_config_path = %path);
            let json: serde_json::Value = serde_json::from_reader(std::fs::File::open(path)?)?;
            let repos: Vec<_> = json.as_object().unwrap().keys().collect();
            let emit = batch_dir.join(path.file_name().unwrap());
            info!(?repos, "checking");
            let mut args = vec![
                "run",
                "--config",
                path.as_str(),
                "--emit",
                emit.as_str(),
                "--db",
                DB,
            ];
            args.extend(batch.os_checker_args.iter().map(|arg| arg.as_str()));
            let expr = cmd("os-checker", &args);
            info!(cmd = ?expr);
            expr.run()?;
            if !batch.no_upload {
                upload_cache()?;
            }
            count_json_file += 1;
            count_repos += repos.len();
        }
    }

    info!(count_json_file, count_repos, "done");

    Ok(())
}

fn upload_cache() -> Result<()> {
    // cmd!("ls", "-alh").run()?;
    let tag = var("TAG_CACHE").unwrap();
    let args = format!("release upload --clobber -R os-checker/database {tag} {DB}");
    cmd("gh", args.split(" ")).run()?;
    info!("Successfully upload {DB}.");
    Ok(())
}

mod logger {
    use tracing_subscriber::{fmt, prelude::*, registry, EnvFilter};

    pub fn init() {
        let fmt_layer = fmt::layer();
        let env_layer = EnvFilter::from_default_env();
        let error_layer = tracing_error::ErrorLayer::default();

        if let Err(err) = registry()
            .with(fmt_layer)
            .with(env_layer)
            .with(error_layer)
            .try_init()
        {
            eprintln!("Logger already init: {err}");
        };

        color_eyre::install().unwrap();
    }
}

fn base_dir() -> Utf8PathBuf {
    var("BASE_DIR").map_or_else(
        |_| camino::absolute_utf8(".").unwrap(),
        |path| {
            if path.starts_with("~") {
                let home = dirs::home_dir().unwrap();
                Utf8PathBuf::from(path.replacen("~", home.to_str().unwrap(), 1))
            } else {
                Utf8PathBuf::from(path)
            }
        },
    )
}
