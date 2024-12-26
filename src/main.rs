#![warn(rust_2018_idioms, missing_debug_implementations, clippy::pedantic)]

use clap::Parser;
use std::{
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::Instant,
};

mod args;

#[tokio::main]
async fn main() {
    let cli_args = match args::Args::parse().validate() {
        Ok(args) => args,
        Err(err) => {
            eprintln!("{err}");
            return;
        }
    };

    let total_requests = Arc::new(AtomicU32::new(0));

    let handles = (0..cli_args.max_concurrent_requests)
        .map(|_| {
            let host = cli_args.host.clone();
            let total_requests = Arc::clone(&total_requests);
            tokio::spawn(async move {
                let mut request_id = total_requests.fetch_add(1, Ordering::Acquire);
                while request_id < cli_args.num_requests {
                    let start = Instant::now();
                    let _ = reqwest::get(host.clone()).await;
                    println!(
                        "Request {}: {}ms",
                        request_id + 1,
                        start.elapsed().as_millis()
                    );
                    request_id = total_requests.fetch_add(1, Ordering::Acquire);
                }
            })
        })
        .collect::<Vec<_>>();

    for h in handles {
        let _ = h.await;
    }
}
