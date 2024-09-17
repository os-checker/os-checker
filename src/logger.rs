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

#[cfg(test)]
#[allow(dead_code)]
pub fn test_init(log_level: Option<&str>, log_file: &str) {
    let is_file = std::env::var("LOG_FILE").is_ok();
    let filter = log_level
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(EnvFilter::from_default_env);
    let fmt = fmt().with_env_filter(filter).without_time();
    let init = if is_file {
        fmt.with_writer(std::fs::File::create(log_file).unwrap())
            .with_ansi(false)
            .try_init()
    } else {
        fmt.try_init()
    };
    if let Err(err) = init {
        eprintln!("Logger already init: {err}");
    };
}
