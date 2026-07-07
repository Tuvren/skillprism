# skillprism — npm launcher

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
- `SKILLPRISM_SKIP_CHECKSUM` — Set to `1` to bypass tarball checksum verification.
  Insecure; intended only for local development against unreleased builds.

## Checksum verification

Downloaded tarballs are verified against `checksums.json` before extraction. The
manifest maps each published version to the SHA-256 of every per-target archive:

```json
{
  "0.1.0": {
    "x86_64-unknown-linux-gnu": "sha256:<hex>",
    "x86_64-apple-darwin": "sha256:<hex>",
    "aarch64-apple-darwin": "sha256:<hex>"
  }
}
```

A `null` value is a placeholder for a target that has **not yet been released**;
the launcher treats a missing/`null` entry as "no published checksum" and refuses
to run that binary (unless `SKILLPRISM_SKIP_CHECKSUM=1`). The release pipeline is
responsible for replacing these placeholders with the real digests emitted
alongside the GitHub Release artifacts before the corresponding npm version is
published. Until a version's entries are populated, `npx skillprism` for that
version will fail closed rather than execute an unverified binary.

## Requirements

- Node.js >= 18 (for `npx`)
- `tar` on PATH (for extracting the downloaded archive)
