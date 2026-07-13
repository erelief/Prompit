import { readFileSync, writeFileSync } from "fs";
import { resolve } from "path";

// ── Sources of truth ────────────────────────────────────────────────────
// package.json.version  →  single source for the app version.
// src/shared/main-window.json.width  →  single source for the main window width.
//
// This script propagates both into src-tauri/tauri.conf.json (and the version
// into src-tauri/Cargo.toml) so Tauri/Cargo see consistent values. It runs
// before every dev/build (via beforeDevCommand/beforeBuildCommand) and from
// the npm `version` lifecycle hook. Idempotent: writes only on change.

const PKG_PATH = resolve("package.json");
const SHARED_PATH = resolve("src/shared/main-window.json");
const TAURI_CONFIG_PATH = resolve("src-tauri/tauri.conf.json");
const CARGO_PATH = resolve("src-tauri/Cargo.toml");

// ── Read sources ─────────────────────────────────────────────────────────
const pkg = JSON.parse(readFileSync(PKG_PATH, "utf-8"));
const targetVersion = pkg.version;
if (typeof targetVersion !== "string" || !targetVersion) {
  console.error(`✗ package.json has no valid "version" field`);
  process.exit(1);
}

const shared = JSON.parse(readFileSync(SHARED_PATH, "utf-8"));
const targetWidth = shared.width;
if (typeof targetWidth !== "number") {
  console.error(`✗ ${SHARED_PATH} must contain a numeric "width" field`);
  process.exit(1);
}

// ── Patch tauri.conf.json (width + version) ──────────────────────────────
const tauriConfig = JSON.parse(readFileSync(TAURI_CONFIG_PATH, "utf-8"));
const currentWidth = tauriConfig?.app?.windows?.[0]?.width;
if (currentWidth === undefined) {
  console.error(`✗ Could not find app.windows[0].width in ${TAURI_CONFIG_PATH}`);
  process.exit(1);
}

let tauriChanged = false;
if (currentWidth !== targetWidth) {
  tauriConfig.app.windows[0].width = targetWidth;
  console.log(`✓ tauri.conf.json width: ${currentWidth} → ${targetWidth}`);
  tauriChanged = true;
}
if (tauriConfig.version !== targetVersion) {
  const prev = tauriConfig.version;
  tauriConfig.version = targetVersion;
  console.log(`✓ tauri.conf.json version: ${prev} → ${targetVersion}`);
  tauriChanged = true;
}
if (tauriChanged) {
  writeFileSync(TAURI_CONFIG_PATH, JSON.stringify(tauriConfig, null, 2) + "\n");
}

// ── Patch Cargo.toml version (the first `version = "..."` line = [package]) ─
let cargo = readFileSync(CARGO_PATH, "utf-8");
// `m` flag → `^` matches line starts; dependency versions live mid-line
// (e.g. `tauri = { version = "2", ... }`) so they are untouched.
const versionRe = /^version\s*=\s*"[^"]*"/m;
const m = cargo.match(versionRe);
if (!m) {
  console.error(`✗ Could not find a standalone version field in ${CARGO_PATH}`);
  process.exit(1);
}
if (m[0] !== `version = "${targetVersion}"`) {
  cargo = cargo.replace(versionRe, `version = "${targetVersion}"`);
  writeFileSync(CARGO_PATH, cargo);
  console.log(`✓ Cargo.toml version → ${targetVersion}`);
}
