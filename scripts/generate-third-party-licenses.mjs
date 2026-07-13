// Generate a consolidated THIRD-PARTY-LICENSES file by parsing the project's
// lock files directly. This avoids requiring external CLI tools (cargo-license,
// license-checker) or network access, so it works offline and in any CI.
//
// Output: public/THIRD-PARTY-LICENSES
//   - npm production dependencies (from package-lock.json, dev deps excluded)
//   - Rust crates (from src-tauri/Cargo.lock)
//
// Only dependency name / version / license / source are listed. For full
// license texts, extend `readLicenseText()` below to read each package's
// LICENSE file from node_modules / the cargo registry cache.

import { readFileSync, writeFileSync, existsSync, readdirSync, readFile, mkdirSync } from "fs";
import { resolve, dirname, join } from "path";
import { fileURLToPath } from "url";
import { promisify } from "util";
import { homedir } from "os";

const readFileAsync = promisify(readFile);
const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");
const OUT = join(ROOT, "public", "THIRD-PARTY-LICENSES");

/** Read a package's license text if a LICENSE file ships in its folder. */
async function readLicenseText(pkgDir) {
  if (!pkgDir || !existsSync(pkgDir)) return null;
  const candidates = readdirSync(pkgDir).filter((f) =>
    /^license(\.[a-z0-9]+)?$/i.test(f)
  );
  if (candidates.length === 0) return null;
  try {
    return await readFileAsync(join(pkgDir, candidates[0]), "utf8");
  } catch {
    return null;
  }
}

/** Locate a crate's extracted source in the cargo registry cache (offline). */
function findCargoCacheDir(name, version) {
  const base = join(homedir(), ".cargo", "registry", "src");
  if (!existsSync(base)) return null;
  for (const hashDir of readdirSync(base)) {
    const dir = join(base, hashDir);
    if (!existsSync(dir)) continue;
    const match = readdirSync(dir).find((d) => d === `${name}-${version}`);
    if (match) return join(dir, match);
  }
  return null;
}

/** Best-effort license identifier from a crate's Cargo.toml (offline). */
function readCargoLicense(name, version) {
  const dir = findCargoCacheDir(name, version);
  if (!dir) return "";
  const tomlPath = join(dir, "Cargo.toml");
  if (!existsSync(tomlPath)) return "";
  const toml = readFileSync(tomlPath, "utf8");
  const m = toml.match(/^\s*license\s*=\s*"([^"]+)"/m);
  return m ? m[1] : "";
}

/** ── npm (production) ── */
function collectNpm(root) {
  const lockPath = join(root, "package-lock.json");
  if (!existsSync(lockPath)) return [];
  const lock = JSON.parse(readFileSync(lockPath, "utf8"));
  const packages = lock.packages ?? {};
  const out = [];

  for (const [key, meta] of Object.entries(packages)) {
    if (key === "") continue; // root project
    if (meta.dev === true) continue; // dev-only dependency, not bundled
    // For nested paths (node_modules/a/node_modules/b) take the last segment group
    const parts = key.split("node_modules/");
    const pkgName = parts[parts.length - 1];
    out.push({
      name: pkgName,
      version: meta.version ?? "",
      license: meta.license ?? "",
      url: `https://www.npmjs.com/package/${pkgName}`,
      pkgDir: join(root, "node_modules", pkgName),
      source: "npm",
    });
  }
  return out;
}

/** ── Rust (Cargo.lock, TOML-ish) ── */
function collectCargo(root) {
  const lockPath = join(root, "src-tauri", "Cargo.lock");
  if (!existsSync(lockPath)) return [];
  const text = readFileSync(lockPath, "utf8");
  const blocks = text.split(/\n\[\[package\]\]\n/).slice(1);
  const out = [];

  for (const block of blocks) {
    const get = (re) => {
      const m = block.match(re);
      return m ? m[1].trim() : "";
    };
    const name = get(/^name\s*=\s*"([^"]+)"/m);
    const version = get(/^version\s*=\s*"([^"]+)"/m);
    if (!name) continue;
    const license = get(/^license\s*=\s*"([^"]+)"/m) || readCargoLicense(name, version);
    out.push({
      name,
      version,
      license,
      url: `https://crates.io/crates/${name}`,
      pkgDir: findCargoCacheDir(name, version),
      source: "crates.io",
    });
  }
  return out;
}

/**
 * ── Curated About-page dependencies ──
 *
 * The About page shows a small, hand-picked list of direct dependencies
 * (not the full transitive tree). Only the *display name* and *lookup key*
 * are authored here — the version is resolved from the lockfile at build
 * time, so this list never goes stale when deps are upgraded.
 *
 *   key:   the package/crate name as it appears in the lockfile
 *   url:   canonical project homepage (manual, since lockfiles only have
 *          registry URLs which are less readable)
 *   source:"npm" | "cargo"
 *
 * To add/remove an entry on the About page, edit this array only.
 */
const ABOUT_DEPS = [
  // ── Frontend (npm) ──
  { key: "@tauri-apps/api", display: "Tauri API", url: "https://tauri.app", source: "npm" },
  { key: "vue", display: "Vue", url: "https://vuejs.org", source: "npm" },
  { key: "vue-router", display: "Vue Router", url: "https://router.vuejs.org", source: "npm" },
  { key: "vue-i18n", display: "Vue I18n", url: "https://vue-i18n.intlify.dev", source: "npm" },
  { key: "@vueuse/core", display: "VueUse", url: "https://vueuse.org", source: "npm" },
  { key: "@lucide/vue", display: "Lucide", url: "https://lucide.dev", source: "npm" },
  { key: "tailwindcss", display: "Tailwind CSS", url: "https://tailwindcss.com", source: "npm" },
  { key: "vuedraggable", display: "VueDraggable", url: "https://sortablejs.github.io/vue.draggable.next/", source: "npm" },
  // ── Rust: core / async / networking ──
  { key: "tauri", display: "Tauri", url: "https://tauri.app", source: "cargo" },
  { key: "tokio", display: "tokio", url: "https://crates.io/crates/tokio", source: "cargo" },
  { key: "reqwest", display: "reqwest", url: "https://crates.io/crates/reqwest", source: "cargo" },
  { key: "serde", display: "serde", url: "https://crates.io/crates/serde", source: "cargo" },
  { key: "serde_json", display: "serde_json", url: "https://crates.io/crates/serde_json", source: "cargo" },
  // OS credential store: binds the vault's local KEK to Windows Credential
  // Manager / macOS Keychain / Linux Secret Service.
  { key: "keyring", display: "Keyring", url: "https://crates.io/crates/keyring", source: "cargo" },
  { key: "tauri-plugin-global-shortcut", display: "tauri-plugin-global-shortcut", url: "https://crates.io/crates/tauri-plugin-global-shortcut", source: "cargo" },
  // ── Rust: cryptography & security (vault) ──
  { key: "argon2", display: "argon2", url: "https://crates.io/crates/argon2", source: "cargo" },
  { key: "aes-gcm", display: "aes-gcm", url: "https://crates.io/crates/aes-gcm", source: "cargo" },
  { key: "sha2", display: "sha2", url: "https://crates.io/crates/sha2", source: "cargo" },
  { key: "rand", display: "rand", url: "https://crates.io/crates/rand", source: "cargo" },
  { key: "zeroize", display: "zeroize", url: "https://crates.io/crates/zeroize", source: "cargo" },
  { key: "base64", display: "base64", url: "https://crates.io/crates/base64", source: "cargo" },
  // ── Rust: system capabilities ──
  { key: "enigo", display: "enigo", url: "https://crates.io/crates/enigo", source: "cargo" },
  { key: "arboard", display: "arboard", url: "https://crates.io/crates/arboard", source: "cargo" },
  { key: "csv", display: "csv", url: "https://crates.io/crates/csv", source: "cargo" },
  { key: "url", display: "url", url: "https://crates.io/crates/url", source: "cargo" },
  { key: "dotenvy", display: "dotenvy", url: "https://crates.io/crates/dotenvy", source: "cargo" },
];

/**
 * Read src-tauri/Cargo.toml and return a map of crate name → version
 * requirement (the leading numeric group only, e.g. "0.22" for `= "0.22"`).
 * Covers [dependencies] and all [target.*.dependencies] tables so
 * platform-specific crates resolve too. Used to disambiguate crates that
 * appear with multiple versions in Cargo.lock.
 */
function readCargoVersionReq() {
  const tomlPath = join(ROOT, "src-tauri", "Cargo.toml");
  if (!existsSync(tomlPath)) return {};
  const text = readFileSync(tomlPath, "utf8");
  const req = {};
  // Walk dependency lines, capturing crate name + version token together.
  // Handles both `name = "1"` and `name = { version = "0.22", ... }` forms.
  for (const line of text.split("\n")) {
    const m = line.match(/^\s*([a-z0-9_-]+)\s*=\s*(?:\{[^}]*?"?version"?\s*=\s*"([^"]+)"|\s*"([^"]+)")/);
    if (m) {
      const name = m[1];
      const raw = m[2] || m[3] || "";
      // Keep only the leading "major.minor" group of the requirement.
      const lead = raw.match(/^=?\s*(\d+(?:\.\d+)?)/);
      if (lead) req[name] = lead[1];
    }
  }
  return req;
}

/**
 * Resolve the curated About deps against the parsed lockfile data, emitting
 * { name, version, url } for each. Entries that can't be resolved (e.g. a
 * conditional dep absent on this platform) are skipped with a warning so the
 * About page never shows a blank version.
 *
 * For cargo crates with multiple locked versions (e.g. base64 0.21.x as a
 * transitive dep AND 0.22.x as our direct dep), the version required by
 * Cargo.toml is preferred over an incidental transitive one.
 */
function resolveAboutDeps(npm, cargo) {
  const byName = (list, name) => list.find((d) => d.name === name);
  // Parse the version requirement each cargo crate is declared with in
  // Cargo.toml (e.g. base64 = "0.22" → "0.22"), so we can disambiguate when a
  // crate appears more than once in Cargo.lock. Returns "" if not declared.
  const cargoReq = readCargoVersionReq();
  function cargoVersionFor(name) {
    const matches = cargo.filter((d) => d.name === name);
    if (matches.length === 0) return null;
    if (matches.length === 1) return matches[0];
    const req = cargoReq[name];
    if (req) {
      // Match by the leading numeric group of the requirement (e.g. "0.22").
      const hit = matches.find((m) => m.version.startsWith(req));
      if (hit) return hit;
    }
    // Fallback: highest version.
    return matches.sort((a, b) => a.version.localeCompare(b.version, undefined, { numeric: true }))[0];
  }
  const out = [];
  for (const dep of ABOUT_DEPS) {
    const found = dep.source === "npm" ? byName(npm, dep.key) : cargoVersionFor(dep.key);
    if (!found || !found.version) {
      console.warn(`[third-party] about-deps: "${dep.key}" not found in lockfile, skipping`);
      continue;
    }
    out.push({ name: dep.display, version: found.version, url: dep.url });
  }
  return out;
}

async function main() {
  const npm = collectNpm(ROOT);
  const cargo = collectCargo(ROOT);

  const all = [...npm, ...cargo]
    .filter((d) => d.name && d.version)
    .sort((a, b) => a.name.localeCompare(b.name));

  const seen = new Set();
  const unique = all.filter((d) => {
    const k = `${d.name}@${d.version}`;
    if (seen.has(k)) return false;
    seen.add(k);
    return true;
  });

  let text = "";
  text += "THIRD-PARTY LICENSES / OPEN SOURCE ATTRIBUTIONS\n";
  text += "================================================\n\n";
  text += "This product bundles the following open-source components.\n";
  text += `Generated ${new Date().toISOString().slice(0, 10)} from package-lock.json and Cargo.lock.\n\n`;

  // Optional: include full license texts when available (npm only).
  const withTexts = [];
  for (const d of unique) {
    let body = "";
    if (d.source === "npm") {
      const lt = await readLicenseText(d.pkgDir);
      if (lt && lt.trim().length > 0) body = lt.trim();
    }
    withTexts.push({ ...d, body });
  }

  for (const d of withTexts) {
    text += "------------------------------------------------------------\n";
    text += `${d.name}  ${d.version}\n`;
    text += `License: ${d.license || "Unknown"}\n`;
    text += `Source:  ${d.url}\n`;
    if (d.body) {
      text += "\n" + d.body + "\n";
    }
    text += "\n";
  }

  const total = unique.length;
  const npmCount = unique.filter((d) => d.source === "npm").length;
  const cargoCount = unique.filter((d) => d.source === "crates.io").length;
  text += "================================================\n";
  text += `Total components: ${total}  (npm: ${npmCount}, crates.io: ${cargoCount})\n`;

  writeFileSync(OUT, text, "utf8");
  console.log(
    `[third-party] wrote ${total} components (npm: ${npmCount}, crates.io: ${cargoCount}) -> public/THIRD-PARTY-LICENSES`
  );

  // Curated About-page dependency list. Written under src/generated/ so it is
  // imported as a module (no runtime fetch, works under Tauri CSP). Versions
  // come straight from the lockfiles, so the About page stays in sync with
  // upgrades automatically.
  const aboutDeps = resolveAboutDeps(npm, cargo);
  const GENERATED_DIR = join(ROOT, "src", "generated");
  mkdirSync(GENERATED_DIR, { recursive: true });
  writeFileSync(
    join(GENERATED_DIR, "about-deps.json"),
    JSON.stringify(aboutDeps, null, 2) + "\n",
    "utf8"
  );
  console.log(`[third-party] wrote ${aboutDeps.length} about-deps -> src/generated/about-deps.json`);
}

main().catch((err) => {
  console.error("[third-party] failed:", err);
  process.exit(1);
});
