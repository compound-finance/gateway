//! Gateway CLI library.
#![warn(missing_docs)]

mod chain_spec;
#[macro_use]
mod service;
mod api;
mod cli;
mod command;
mod rpc;

fn main() -> sc_cli::Result<()> {
    command::run()
}
