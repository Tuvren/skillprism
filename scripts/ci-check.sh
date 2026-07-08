#!/usr/bin/env bash
set -euo pipefail

# Reproduce the GitHub Actions CI checks locally on the *pinned* MSRV toolchain
# (.github/workflows/ci.yml + rust-toolchain.toml → 1.85), exactly as CI runs
# them: build/test/clippy/fmt with --all-targets and -D warnings.
#
# Why this exists: a local rustc newer than 1.85 — e.g. a source-built toolchain
# that ignores rust-toolchain.toml, or a devenv/nixpkgs rust — can pass clippy
# while CI's 1.85 fails. clippy *nursery*/*pedantic* lints vary between toolchain
# versions, and the crate denies them (#![deny(clippy::nursery)]), so a version
# gap surfaces only in CI. Run this before pushing to catch it locally.
#
# Override the toolchain with SKILLPRISM_CI_TOOLCHAIN if CI's pin changes.

cd "$(git rev-parse --show-toplevel 2>/dev/null || echo "$(dirname "$0")/..")"

TOOLCHAIN="${SKILLPRISM_CI_TOOLCHAIN:-1.85}"

if ! command -v rustup >/dev/null 2>&1; then
    echo "error: rustup is required to pin the CI toolchain ($TOOLCHAIN)." >&2
    echo "       Install rustup, or run the four cargo steps below manually" >&2
    echo "       with a $TOOLCHAIN toolchain." >&2
    exit 1
fi

if ! rustup toolchain list | grep -q "^${TOOLCHAIN}"; then
    echo "Installing missing toolchain ${TOOLCHAIN}..." >&2
    rustup toolchain install "$TOOLCHAIN" --component clippy rustfmt
fi

run() {
    echo "+ rustup run $TOOLCHAIN $*"
    rustup run "$TOOLCHAIN" "$@"
}

run cargo build --locked --all-targets
run cargo test --locked
run cargo clippy --all-targets -- -D warnings
run cargo fmt --check

echo "CI parity checks passed on toolchain ${TOOLCHAIN}."
