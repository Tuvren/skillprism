#!/usr/bin/env bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel 2>/dev/null || echo "$(dirname "$0")/..")"

cargo build --release --quiet 2>&1 || exit 1
./target/release/skillprism __generate_man > skillprism.1 || {
    echo "Error: failed to generate man page" >&2
    exit 1
}

echo "Generated skillprism.1 man page"
