#!/usr/bin/env node

// Thin launcher that downloads and executes the correct skillprism binary
// for the current platform from the latest GitHub Release.
//
// Pattern: Biome (@biomejs/biome), esbuild, Playwright.
// The Rust binary is the real artifact — this package is just a gateway.

import { createWriteStream, existsSync, mkdirSync, renameSync, chmodSync, rmSync } from "node:fs";
import { unlink } from "node:fs/promises";
import { homedir } from "node:os";
import { join } from "node:path";
import { spawn } from "node:child_process";
import { pipeline } from "node:stream/promises";

const REPO = "tuvren/skillprism";
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

  mkdirSync(CACHE_DIR, { recursive: true });

  // NOTE: This downloads a release artifact from GitHub over HTTPS and runs it.
  // Robust supply-chain verification (signed checksum manifest, pinned hashes,
  // or code signing) is not implemented yet. Track progress in the roadmap:
  // https://github.com/Tuvren/skillprism/issues
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`Failed to download binary: ${res.status} ${res.statusText}`);
  }

  const fileStream = createWriteStream(tmpFile);
  await pipeline(res.body, fileStream);

  // Extract binary from the tarball (xz-compressed, no outer dir flattening)
  // The tarball contains: skillprism-<target>/skillprism
  mkdirSync(join(CACHE_DIR, `extract-${version}-${target}`), { recursive: true });

  await extractTarXz(tmpFile, join(CACHE_DIR, `extract-${version}-${target}`));

  // Move binary to final path
  const extractedBinary = join(
    CACHE_DIR,
    `extract-${version}-${target}`,
    `skillprism-${target}`,
    "skillprism",
  );
  const finalPath = getBinaryPath(version);

  renameSync(extractedBinary, finalPath);
  chmodSync(finalPath, 0o755);

  // Cleanup
  await unlink(tmpFile);
  await rmRecursive(join(CACHE_DIR, `extract-${version}-${target}`));

  return finalPath;
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

async function rmRecursive(dir) {
  const { rmSync } = await import("node:fs");
  rmSync(dir, { recursive: true, force: true });
}

async function run() {
  const version = process.env.SKILLPRISM_VERSION || (await getLatestVersion());
  const binaryPath = getBinaryPath(version);

  if (!existsSync(binaryPath)) {
    console.error(`Downloading skillprism v${version} for ${getTarget()}...`);
    await downloadBinary(version);
  }

  const child = spawn(binaryPath, process.argv.slice(2), {
    stdio: "inherit",
  });

  child.on("exit", (code) => {
    process.exit(code);
  });
}

run().catch((err) => {
  console.error("skillprism launcher error:", err.message);
  process.exit(1);
});
