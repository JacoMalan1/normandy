#![deny(missing_docs)]

use clap::Parser;
use thiserror::Error;

/// A simple CLI tool for load-testing webservers.
#[derive(Debug, Clone, Parser)]
#[command(version, about)]
pub struct Args {
    /// Maximum number of concurrent requests.
    #[arg(short = 'c', default_value = "10")]
    max_concurrent_requests: u32,
    /// Total number of requests to send.
    #[arg(short = 'n')]
    num_requests: u32,
    host: String,
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Invalid host url.")]
    InvalidHostUrl,
}

impl Args {
    pub fn validate(&self) -> Result<Validated, ValidationError> {
        Ok(Validated {
            host: self
                .host
                .parse()
                .map_err(|_| ValidationError::InvalidHostUrl)?,
            max_concurrent_requests: self.max_concurrent_requests,
            num_requests: self.num_requests,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Validated {
    pub host: reqwest::Url,
    pub max_concurrent_requests: u32,
    pub num_requests: u32,
}
