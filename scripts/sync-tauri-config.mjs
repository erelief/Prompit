import { readFileSync, writeFileSync } from "fs";
import { resolve } from "path";

const SHARED_PATH = resolve("src/shared/main-window.json");
const TAURI_CONFIG_PATH = resolve("src-tauri/tauri.conf.json");

// Read shared window config
let shared;
try {
  shared = JSON.parse(readFileSync(SHARED_PATH, "utf-8"));
} catch (err) {
  console.error(`❌ Failed to read ${SHARED_PATH}: ${err.message}`);
  process.exit(1);
}

const targetWidth = shared.width;
if (typeof targetWidth !== "number") {
  console.error(`❌ ${SHARED_PATH} must contain a numeric "width" field`);
  process.exit(1);
}

// Read and patch tauri.conf.json
let tauriConfig;
try {
  tauriConfig = JSON.parse(readFileSync(TAURI_CONFIG_PATH, "utf-8"));
} catch (err) {
  console.error(`❌ Failed to read ${TAURI_CONFIG_PATH}: ${err.message}`);
  process.exit(1);
}

const currentWidth = tauriConfig?.app?.windows?.[0]?.width;
if (currentWidth === undefined) {
  console.error(`❌ Could not find app.windows[0].width in ${TAURI_CONFIG_PATH}`);
  process.exit(1);
}

if (currentWidth === targetWidth) {
  console.log(`✓ tauri.conf.json width already ${targetWidth} — no change needed`);
  process.exit(0);
}

tauriConfig.app.windows[0].width = targetWidth;
writeFileSync(TAURI_CONFIG_PATH, JSON.stringify(tauriConfig, null, 2) + "\n");
console.log(`✓ synced tauri.conf.json width: ${currentWidth} → ${targetWidth}`);