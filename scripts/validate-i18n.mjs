import { readFileSync, readdirSync } from "fs";
import { resolve, join } from "path";

const LOCALES_DIR = resolve("src/locales");

/**
 * Recursively extract all key paths from a nested object.
 * e.g. { a: { b: "x" } } → ["a.b"]
 */
function extractKeys(obj, prefix = "") {
  const keys = [];
  for (const [k, v] of Object.entries(obj)) {
    const path = prefix ? `${prefix}.${k}` : k;
    if (typeof v === "object" && v !== null && !Array.isArray(v)) {
      keys.push(...extractKeys(v, path));
    } else {
      keys.push(path);
    }
  }
  return keys;
}

// Read en.json as the source of truth
const enPath = join(LOCALES_DIR, "en.json");
let enData;
try {
  enData = JSON.parse(readFileSync(enPath, "utf-8"));
} catch (err) {
  console.error(`❌ Failed to read ${enPath}: ${err.message}`);
  process.exit(1);
}

const enKeys = new Set(extractKeys(enData));
console.log(`✅ en.json: ${enKeys.size} keys`);

// Check all other locale files
const files = readdirSync(LOCALES_DIR).filter(
  (f) => f.endsWith(".json") && f !== "en.json"
);

let hasError = false;

for (const file of files) {
  const filePath = join(LOCALES_DIR, file);
  let data;
  try {
    data = JSON.parse(readFileSync(filePath, "utf-8"));
  } catch (err) {
    console.error(`❌ Failed to read ${file}: ${err.message}`);
    hasError = true;
    continue;
  }

  const localeKeys = new Set(extractKeys(data));
  const missing = [...enKeys].filter((k) => !localeKeys.has(k));
  const extra = [...localeKeys].filter((k) => !enKeys.has(k));

  if (missing.length > 0) {
    console.error(`\n❌ ${file} is missing ${missing.length} key(s):`);
    for (const k of missing) console.error(`   - ${k}`);
    hasError = true;
  }

  if (extra.length > 0) {
    console.warn(`\n⚠️  ${file} has ${extra.length} extra key(s) not in en.json:`);
    for (const k of extra) console.warn(`   - ${k}`);
  }

  if (missing.length === 0 && extra.length === 0) {
    console.log(`✅ ${file}: all ${localeKeys.size} keys present`);
  }
}

if (hasError) {
  console.error("\n💥 i18n validation failed. Fix missing keys before building.");
  process.exit(1);
} else {
  console.log("\n🎉 All locale files are complete.");
}
