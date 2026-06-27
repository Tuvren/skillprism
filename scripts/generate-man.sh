#!/usr/bin/env bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel 2>/dev/null || echo "$(dirname "$0")/..")"

cargo run -- __generate_man > skillprism.1 2>/dev/null

echo "Generated skillprism.1 man page"
