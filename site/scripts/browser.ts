import { execSync } from "child_process";

/**
 * Resolve a Chromium-family browser binary for `puppeteer.launch({ executablePath })`.
 *
 * Resolution order:
 *   1. `$BROWSER` env override (an explicit path or a binary name on PATH)
 *   2. common browser binaries found on PATH
 *   3. `undefined` — lets puppeteer fall back to its own bundled Chromium
 *
 * This avoids hardcoding a single browser (previously `brave`), which threw on
 * any machine where the browser is chromium/chrome instead.
 */
export function resolveBrowser(): string | undefined {
  const override = process.env.BROWSER?.trim();
  if (override) return override;

  const candidates = [
    "brave",
    "brave-browser",
    "chromium",
    "chromium-browser",
    "google-chrome-stable",
    "google-chrome",
    "chrome",
  ];

  for (const bin of candidates) {
    try {
      const path = execSync(`command -v ${bin}`, { encoding: "utf8" }).trim();
      if (path) return path;
    } catch {
      // not on PATH — try the next candidate
    }
  }

  return undefined;
}
