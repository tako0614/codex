#!/usr/bin/env node
// Postinstall script to set execute permissions on binaries

import { chmod } from "node:fs/promises";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const vendorRoot = path.join(__dirname, "..", "vendor");

const binaries = [
  "x86_64-unknown-linux-musl/codex/codex",
  "aarch64-unknown-linux-musl/codex/codex",
  "x86_64-apple-darwin/codex/codex",
  "aarch64-apple-darwin/codex/codex",
];

for (const binary of binaries) {
  const binaryPath = path.join(vendorRoot, binary);
  if (existsSync(binaryPath)) {
    try {
      await chmod(binaryPath, 0o755);
    } catch (err) {
      // Ignore errors (e.g., on Windows)
    }
  }
}
