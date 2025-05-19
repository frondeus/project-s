#![deny(clippy::unwrap_used)]

pub type Result<T = (), E = anyhow::Error> = std::result::Result<T, E>;

mod runner;

pub use runner::test_snapshots;

mod utils;
