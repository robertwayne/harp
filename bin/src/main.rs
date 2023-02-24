#![forbid(unsafe_code)]
#![feature(vec_push_within_capacity)]

pub mod config;
pub mod server;

use std::process::exit;

use harp::Result;
use pico_args::Arguments;
use tracing::metadata::LevelFilter;

use crate::config::Config;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const HELP: &str = "\
harpd {VERSION}
Rob Wagner <rob@sombia.com>
https://github.com/robertwayne/harp

USAGE:
    harpd [OPTIONS]

OPTIONS:
    -c, --config <FILE>    Sets a custom config file
    -h, --help             Displays help information
    -v, --version          Displays version information
";

#[derive(Debug)]
struct Args {
    config_path: Option<String>,
}

fn parse_args(help: &str) -> Result<Args> {
    let mut pargs = Arguments::from_env();

    if pargs.contains(["-h", "--help"]) {
        println!("{help}");
        exit(0);
    }

    if pargs.contains(["-v", "--version"]) {
        println!("harpd {}", env!("CARGO_PKG_VERSION"));
        exit(0);
    }

    let args = Args { config_path: pargs.opt_value_from_str(["-c", "--config"])? };

    let remaining = pargs.finish();
    if !remaining.is_empty() {
        println!("Unknown arguments: {:?}\n\n{help}", remaining);
        exit(1);
    }

    Ok(args)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::SubscriberBuilder::default()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    // Unfortunately, I don't see a way to format a string literal in a const
    // context currently.
    let help = HELP.replace("{VERSION}", VERSION);

    let args = match parse_args(&help) {
        Ok(args) => args,
        Err(_) => {
            println!("{help}");
            exit(1);
        }
    };

    let config = Config::load_from_file(args.config_path)?;

    let pg = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.get_database_url())
        .await?;

    // TODO: The migration files need to be embed in the binary at build time.
    sqlx::migrate!().run(&pg).await?;

    if let Err(e) = server::listen(config, pg).await {
        tracing::error!("Error listening: {e}");
        exit(1);
    }

    Ok(())
}
