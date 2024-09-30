// #!/usr/bin/env -S cargo +nightly -Zscript

#[macro_use]
extern crate eyre;

#[macro_use]
extern crate tracing;

use argh::FromArgs;
use camino::Utf8PathBuf;
use color_eyre::Result;
use duct::cmd;
use std::env::var;

#[derive(FromArgs)]
/// Run os-checker in batch.
struct Batch {
    /// forward --size
    #[argh(option)]
    size: String,
}

#[instrument(level = "trace")]
fn main() -> Result<()> {
    logger::init();
    let batch: Batch = argh::from_env();

    let base_dir = base_dir();
    std::env::set_current_dir(&base_dir)?;
    info!(%base_dir, "set_current_dir");
    let config_dir = base_dir.join("config");
    let batch_dir = base_dir.join("batch");

    let configs: Vec<_> = var("CONFIGS")?
        .trim()
        .split(" ")
        .map(Utf8PathBuf::from)
        .collect();
    info!(?configs);
    ensure!(!configs.is_empty(), "CONFIGS env var should be specified.");
    for config in &configs {
        ensure!(config.exists(), "{config} does not exists.");
    }
    let arg_configs = configs.iter().flat_map(|c| ["--config", c.as_str()]);

    let mut args = Vec::<&str>::with_capacity(16);
    args.push("batch");
    args.extend(arg_configs);
    args.extend(["--out-dir", config_dir.as_str()]);
    args.extend(["--size", &batch.size]);
    cmd("os-checker", args).run()?;

    let [mut count_json_file, mut count_repos] = [0usize; 2];
    for entry in config_dir.read_dir_utf8()? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_file() && path.extension() == Some("json") {
            info!(batch_config_path = %path);
            let json: serde_json::Value = serde_json::from_reader(std::fs::File::open(path)?)?;
            let repos: Vec<_> = json.as_object().unwrap().keys().collect();
            let emit = batch_dir.join(path.file_stem().unwrap());
            info!(?repos, "checking");
            let expr = cmd!(
                "os-checker",
                "run",
                "--config",
                path.as_str(),
                "--emit",
                emit.as_str(),
                "--db",
                "cache.redb"
            );
            info!(cmd = ?expr);
            expr.run()?;
            upload_cache()?;
            count_json_file += 1;
            count_repos += repos.len();
        }
    }

    info!(count_json_file, count_repos, "done");

    Ok(())
}

fn upload_cache() -> Result<()> {
    cmd!("ls", "-alh").run()?;
    let args = "release upload --clobber -R os-checker/database cache.redb cache.redb".split(" ");
    cmd("gh", args).run()?;
    info!("Successfully upload cache.redb.");
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
    let home = Utf8PathBuf::from_path_buf(dirs::home_dir().unwrap()).unwrap();
    var("BASE_DIR").map_or_else(
        |_| home.join("check"),
        |path| {
            if path.starts_with("~") {
                Utf8PathBuf::from(path.replacen("~", home.as_str(), 1))
            } else {
                Utf8PathBuf::from(path)
            }
        },
    )
}
