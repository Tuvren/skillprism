// Copyright 2026 Oscar Yáñez Cisterna (@SkrOYC)
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

mod cli;
mod distribution;
mod engine;
mod loader;
mod registry;
mod resolver;
mod router;
mod scaffold;
mod state;
mod types;
mod validator;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "__generate_man" {
        if let Err(e) = cli::generate_man_page() {
            eprintln!("{e:?}");
            std::process::exit(1);
        }
        return;
    }
    cli::run();
}
