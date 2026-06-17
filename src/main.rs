#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

mod cli;
mod loader;
mod registry;
mod types;

fn main() {
    cli::run();
}
