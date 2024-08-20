#[macro_use]
extern crate eyre;

#[macro_use]
extern crate tracing;

use compact_str::CompactString as XString;
use eyre::Result;

/// cli argument parsing
mod cli;
/// figure out the codebase layout
mod layout;
/// initialization of logger
mod logger;
/// JSON output: see `../assets/JSON-data-format.md` for more information
mod output;
/// parse yaml file for repo configuration
mod repo;
/// run checker tools based on codebase layout and configuration
mod run_checker;
/// some helper functions
mod utils;

fn main() -> Result<()> {
    logger::init();
    cli::args().run()
}
