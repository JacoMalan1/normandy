#![warn(
    rust_2018_idioms,
    clippy::unwrap_used,
    missing_debug_implementations,
    clippy::pedantic
)]
#![allow(clippy::cast_precision_loss)]

use clap::Parser;
use std::num::NonZeroUsize;

mod args;
mod config;
#[macro_use]
mod logger;
mod worker;

#[tokio::main]
async fn main() {
    let cli_args = match args::Args::parse().validate() {
        Ok(args) => args,
        Err(err) => {
            log!("Error: {err}");
            return;
        }
    };

    if cli_args.verbose {
        logger::global_mut().set_verbose(true);
    }

    let config_contents = tokio::fs::read_to_string("./normandy.ron")
        .await
        .expect("Failed to read `normandy.ron`.");

    let config =
        ron::from_str::<config::Config>(&config_contents).expect("Failed to parse config file");

    let config = config.validate().expect("Failed to validate config");

    verbose!("Config: {config:#?}");
    let num_threads =
        std::thread::available_parallelism().expect("Failed to query number of threads.");
    let num_threads =
        if let Some(max_reqs) = NonZeroUsize::new(cli_args.max_concurrent_requests as usize) {
            num_threads.min(max_reqs)
        } else {
            num_threads
        };

    log!("========================================");
    log!("\tStarting {} worker threads...", num_threads.get());
    log!("========================================\n");
    let mut pool = worker::Pool::new(num_threads, &cli_args.host);
    for req in config
        .requests()
        .iter()
        .cycle()
        .take(cli_args.num_requests as usize)
    {
        let cmd = worker::Command::Request(req.clone());
        verbose!("Sending worker command: {cmd:#?}");
        pool.send_command(cmd).await;
    }

    let mut durations = vec![];
    for _ in 0..cli_args.num_requests {
        if let Some(res) = pool.get_response().await {
            durations.push(res.duration().as_millis());
            log!("{res}");
        } else {
            break;
        }
    }

    let ave = durations.iter().copied().sum::<u128>() as f64 / durations.len() as f64;
    let stddev = (durations
        .iter()
        .copied()
        .map(|x| (x as f64 - ave).powi(2))
        .sum::<f64>()
        / durations.len() as f64)
        .sqrt();

    log!("\n========================================");
    log!("\nAverage request duration: {ave:.2}ms");
    log!("Standard deviation: {stddev:.2}ms");
}
