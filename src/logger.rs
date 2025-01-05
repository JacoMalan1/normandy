#![allow(dead_code)]

use std::sync::{PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

static GLOBAL_LOGGER: RwLock<Logger> = RwLock::new(Logger::new());

#[allow(dead_code)]
pub fn set_global(logger: Logger) {
    *GLOBAL_LOGGER
        .write()
        .unwrap_or_else(PoisonError::into_inner) = logger;
}

pub fn global<'a>() -> RwLockReadGuard<'a, Logger> {
    GLOBAL_LOGGER.read().unwrap_or_else(PoisonError::into_inner)
}

pub fn global_mut<'a>() -> RwLockWriteGuard<'a, Logger> {
    GLOBAL_LOGGER
        .write()
        .unwrap_or_else(PoisonError::into_inner)
}

macro_rules! log {
    ( $fmt:literal $( ,$expr:expr )* $(,)?) => {
        $crate::logger::global().log_args(format_args!($fmt, $($expr),*))
    };
}

macro_rules! verbose {
    ( $fmt:literal $( ,$expr:expr )* $(,)?) => {
        $crate::logger::global().verbose_args(format_args!($fmt, $($expr),*))
    };
}

#[derive(Debug)]
pub struct Logger {
    verbose: bool,
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

impl Logger {
    pub const fn new() -> Self {
        Self { verbose: false }
    }

    pub fn verbose(&self) -> bool {
        self.verbose
    }

    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }

    #[allow(clippy::unused_self)]
    pub fn log_args(&self, args: std::fmt::Arguments<'_>) {
        println!("{args}");
    }

    pub fn verbose_args(&self, args: std::fmt::Arguments<'_>) {
        if self.verbose {
            println!("{args}");
        }
    }
}
