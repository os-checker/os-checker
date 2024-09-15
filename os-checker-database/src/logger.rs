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
