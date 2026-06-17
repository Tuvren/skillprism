#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

mod cli;
mod loader;
mod registry;
mod resolver;
mod types;
mod validator;

fn main() {
    cli::run();
}
