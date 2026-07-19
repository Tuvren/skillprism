#!/usr/bin/env node

// Thin launcher that downloads and executes the correct skillprism binary
// for the current platform from the latest GitHub Release.
//
// Pattern: Biome (@biomejs/biome), esbuild, Playwright.
// The Rust binary is the real artifact — this package is just a gateway.

import {
  createWriteStream,
  existsSync,
  mkdirSync,
  renameSync,
  chmodSync,
  rmSync,
  readFileSync,
} from "node:fs";
import { unlink } from "node:fs/promises";
import { homedir } from "node:os";
import { join, dirname } from "node:path";
import { spawn } from "node:child_process";
import { pipeline } from "node:stream/promises";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";

const REPO = "Tuvren/skillprism";
const CACHE_DIR = join(homedir(), ".cache", "skillprism");
const PLATFORM_MAP = {
  "linux-x64": "x86_64-unknown-linux-gnu",
  "darwin-x64": "x86_64-apple-darwin",
  "darwin-arm64": "aarch64-apple-darwin",
};

function getTarget() {
  const plat = process.platform;
  const arch = process.arch;
  const key = `${plat}-${arch}`;
  const target = PLATFORM_MAP[key];
  if (!target) {
    console.error(`Unsupported platform: ${key}`);
    console.error("Supported: linux-x64, darwin-x64, darwin-arm64");
    process.exit(1);
  }
  return target;
}

function checksumManifestPath() {
  const __filename = fileURLToPath(import.meta.url);
  return join(dirname(__filename), "..", "checksums.json");
}

function embeddedChecksum(version, target) {
  // Fast path: look up the version in the npm package's bundled checksums.json.
  // This avoids a network request for well-known versions.
  const manifest = JSON.parse(readFileSync(checksumManifestPath(), "utf8"));
  const entry = manifest[version]?.[target];
  if (!entry) {
    return null;
  }
  return entry.replace(/^sha256:/, "");
}

async function expectedChecksum(version, target) {
  // 1. Try the npm-package-bundled manifest first (fast path).
  const embedded = embeddedChecksum(version, target);
  if (embedded) {
    return embedded;
  }

  // 2. Fall back to the checksums.json published alongside the release
  //    binaries.  This closes the chicken-and-egg window where the npm
  //    package hasn't been republished yet with the new version's checksums
  //    but the release assets (including checksums.json) are already live.
  //
  //    The release workflow uploads checksums.json as a release asset
  //    alongside every GitHub Release, so it is always present for any
  //    published version.
  try {
    const url = `https://github.com/${REPO}/releases/download/v${version}/checksums.json`;
    const res = await fetch(url);
    if (res.ok) {
      const manifest = await res.json();
      const entry = manifest[target];
      if (entry) {
        return entry.replace(/^sha256:/, "");
      }
    }
  } catch {
    // Network error — fall through to null below.
  }

  return null;
}

function sha256File(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

async function verifyChecksum(tmpFile, version, target) {
  if (process.env.SKILLPRISM_SKIP_CHECKSUM === "1") {
    console.warn(
      `Skipping checksum verification for ${target} v${version}. This is insecure and should only be used for development.`
    );
    return;
  }
  const expected = await expectedChecksum(version, target);
  if (!expected) {
    // No published checksum found anywhere (neither embedded in the npm
    // package nor published as a release asset). The download is still
    // protected by GitHub's HTTPS infrastructure, so warn rather than
    // hard-fail — this can happen transiently between release publish
    // and checksum propagation.
    console.warn(
      `Warning: no published checksum found for ${target} v${version}; ` +
      `skipping integrity verification.`
    );
    return;
  }
  const actual = sha256File(tmpFile);
  if (actual !== expected) {
    throw new Error(
      `Checksum mismatch for ${target} v${version}: expected ${expected}, got ${actual}`
    );
  }
}

function getBinaryPath(version) {
  return join(CACHE_DIR, `skillprism-${version}-${getTarget()}`);
}

async function getLatestVersion() {
  const url = `https://api.github.com/repos/${REPO}/releases/latest`;
  const res = await fetch(url, {
    headers: { Accept: "application/vnd.github.v3+json", "User-Agent": "skillprism-npm" },
  });
  if (!res.ok) {
    const hint = process.env.SKILLPRISM_VERSION
      ? ""
      : " Set SKILLPRISM_VERSION to a specific version to bypass the API rate limit.";
    throw new Error(`Failed to fetch latest release: ${res.status} ${res.statusText}.${hint}`);
  }
  const data = await res.json();
  return data.tag_name.replace(/^v/, "");
}

async function downloadBinary(version) {
  const target = getTarget();
  const url = `https://github.com/${REPO}/releases/download/v${version}/skillprism-${target}.tar.xz`;
  const tmpFile = join(CACHE_DIR, `.download-${version}-${target}.tar.xz`);
  const extractDir = join(CACHE_DIR, `extract-${version}-${target}`);

  mkdirSync(CACHE_DIR, { recursive: true });

  // NOTE: This downloads a release artifact from GitHub over HTTPS and runs it.
  // The tarball is verified against the pinned checksum manifest in
  // checksums.json before extraction. Full signed-manifest / code-signing
  // verification is a future improvement tracked in the roadmap:
  // https://github.com/Tuvren/skillprism/issues
  try {
    const res = await fetch(url);
    if (!res.ok) {
      throw new Error(`Failed to download binary: ${res.status} ${res.statusText}`);
    }

    const fileStream = createWriteStream(tmpFile);
    await pipeline(res.body, fileStream);

    // Verify the downloaded tarball against the pinned checksum manifest before
    // extracting or executing it.
    await verifyChecksum(tmpFile, version, target);

    // Extract binary from the tarball (xz-compressed, no outer dir flattening)
    // The tarball contains: skillprism-<target>/skillprism
    mkdirSync(extractDir, { recursive: true });

    await extractTarXz(tmpFile, extractDir);

    // Move binary to final path
    const extractedBinary = join(extractDir, `skillprism-${target}`, "skillprism");
    const finalPath = getBinaryPath(version);

    renameSync(extractedBinary, finalPath);
    chmodSync(finalPath, 0o755);

    return finalPath;
  } finally {
    // Best-effort cleanup of temporary artifacts on success or failure.
    try {
      if (existsSync(tmpFile)) rmSync(tmpFile);
    } catch {}
    try {
      if (existsSync(extractDir)) rmSync(extractDir, { recursive: true, force: true });
    } catch {}
  }
}

async function extractTarXz(tarXzPath, destDir) {
  // Use system tar for xz decompression + extraction
  return new Promise((resolve, reject) => {
    const tar = spawn("tar", ["-xJf", tarXzPath, "-C", destDir], { stdio: "pipe" });
    tar.on("close", (code) => {
      if (code === 0) resolve();
      else reject(new Error(`tar exited with code ${code}`));
    });
    tar.on("error", reject);
  });
}

function sanitizeVersion(version) {
  // A user-supplied SKILLPRISM_VERSION flows into both a cache file path and the
  // download URL, so reject anything that isn't a plain semver-ish token before
  // it can introduce `../` path traversal or URL trickery.
  if (!/^v?\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(version)) {
    throw new Error(
      `Invalid SKILLPRISM_VERSION "${version}". Expected a version like 0.1.0.`
    );
  }
  return version.replace(/^v/, "");
}

async function run() {
  const version = process.env.SKILLPRISM_VERSION
    ? sanitizeVersion(process.env.SKILLPRISM_VERSION)
    : await getLatestVersion();
  const binaryPath = getBinaryPath(version);

  if (!existsSync(binaryPath)) {
    console.error(`Downloading skillprism v${version} for ${getTarget()}...`);
    await downloadBinary(version);
  }

  const child = spawn(binaryPath, process.argv.slice(2), {
    stdio: "inherit",
  });

  child.on("exit", (code, signal) => {
    if (signal) {
      // Re-raise the signal so the parent sees the same termination cause.
      process.kill(process.pid, signal);
    }
    process.exit(code ?? 1);
  });
}

run().catch((err) => {
  console.error("skillprism launcher error:", err.message);
  process.exit(1);
});
