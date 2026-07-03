# @tuvren/skillprism — npm launcher

This package is a thin downloader gateway for the `skillprism` CLI. The real
binary is a compiled Rust artifact published via GitHub Releases.

## Usage

```bash
npx skillprism add owner/repo
npx skillprism list
npx skillprism --help
```

Or install globally:

```bash
npm install -g skillprism
skillprism add owner/repo
```

## How it works

1. Detects your platform (linux-x64, darwin-x64, darwin-arm64)
2. Downloads the correct pre-built binary from the latest GitHub Release
3. Caches it at `~/.cache/skillprism/`
4. Executes the binary with all forwarded arguments

## Environment variables

- `SKILLPRISM_VERSION` — Pin a specific version (e.g., `0.1.0`). Defaults to latest.

## Requirements

- Node.js >= 18 (for `npx`)
- `tar` on PATH (for extracting the downloaded archive)
