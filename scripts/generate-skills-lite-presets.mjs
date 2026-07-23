// Compile the preset Skills Lite entries from skills-lite-presets/*.md into a
// typed TypeScript module the app imports at runtime. Each .md uses the same
// plaintext template as the user-facing export/import format
// (docs/guides/SKILL.md), so the parsing here mirrors the Rust parser
// `parse_skill_markdown` / `parse_yaml_kv` in
// src-tauri/src/commands/skills_lite.rs byte-for-byte in behavior.
//
// Output: src/generated/skills-lite-presets.ts  (gitignored)
//
// The manifest (scripts/skills-lite-presets.manifest.json, alongside this
// generator) is an ordered list of file stems. ORDER MATTERS: the first entry
// is the default-enabled skill (`enabled: true`); the rest are `enabled: false`.
// This mirrors the existing "the first enabled entry is the one selected by
// default" runtime convention.
//
// To add or modify a preset skill: edit the .md file in skills-lite-presets/
// (and scripts/skills-lite-presets.manifest.json if adding/removing/reordering).
// Re-run `npm run prepare` (or any build) to regenerate the bundle.

import { readFileSync, writeFileSync, mkdirSync, existsSync } from "fs";
import { resolve, dirname, join } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");
const PRESET_DIR = join(ROOT, "skills-lite-presets");
const MANIFEST = join(__dirname, "skills-lite-presets.manifest.json");
const OUT_DIR = join(ROOT, "src", "generated");
const OUT = join(OUT_DIR, "skills-lite-presets.ts");

// ── Markdown parser (mirror of src-tauri/.../skills_lite.rs) ──

/** Parse a `key: value` YAML line, returning the unquoted value if `key` matches. */
function parseYamlKv(line, key) {
  const trimmed = line.trim();
  const prefix = `${key}:`;
  if (!trimmed.startsWith(prefix)) return null;
  let v = trimmed.slice(prefix.length).trim();
  // Strip one pair of matching surrounding quotes. Single-quoted YAML scalars
  // escape embedded quotes by doubling them; collapse back to one.
  if (v.startsWith("'") && v.endsWith("'")) {
    v = v.slice(1, -1).replace(/''/g, "'");
  } else if (v.startsWith('"') && v.endsWith('"')) {
    v = v.slice(1, -1);
  }
  return v;
}

/**
 * Parse the docs/SKILL.md plaintext template into a partial skill entry.
 *
 * Accepts:
 * - An optional YAML frontmatter block delimited by leading `---` lines,
 *   containing `name:` and `description:` fields.
 * - A body whose first `# ` heading sets the name (when frontmatter is absent
 *   or omits `name:`) and the text after that heading becomes the prompt.
 *
 * Returns `null` when no name can be derived.
 */
function parseSkillMarkdown(raw) {
  // Strip UTF-8 BOM if present.
  const content = raw.startsWith("\uFEFF") ? raw.slice(1) : raw;
  const lines = content.split(/\r\n|\r|\n/);

  let name = null;
  let description = null;
  let bodyStart = 0;

  // Frontmatter: first line is exactly "---", find the matching closer.
  if (lines[0]?.trimEnd() === "---") {
    for (let i = 1; i < lines.length; i++) {
      if (lines[i].trimEnd() === "---") {
        // Parse the frontmatter lines [1..i) as `key: value`.
        for (let j = 1; j < i; j++) {
          const n = parseYamlKv(lines[j], "name");
          if (n != null) name = n;
          else {
            const d = parseYamlKv(lines[j], "description");
            if (d != null) description = d;
          }
        }
        bodyStart = i + 1;
        break;
      }
    }
  }

  // Body: skip blank lines, then find the first `# ` heading.
  let bodyLines = lines.slice(bodyStart);
  while (bodyLines.length > 0 && bodyLines[0].trim() === "") {
    bodyLines = bodyLines.slice(1);
  }

  let prompt = "";
  let headingName = null;
  for (let i = 0; i < bodyLines.length; i++) {
    const line = bodyLines[i];
    const m = line.trimStart().match(/^# (.*)$/);
    if (m) {
      headingName = m[1].trim();
      // Prompt is everything after the heading line, blank lines trimmed.
      prompt = bodyLines.slice(i + 1).join("\n").trim();
      break;
    }
  }

  const finalName = (name ?? headingName)?.trim();
  if (!finalName) return null;

  return {
    name: finalName,
    description: (description ?? "").trim(),
    prompt,
  };
}

// ── Generator ──

function main() {
  if (!existsSync(MANIFEST)) {
    console.error(`[skills-lite-presets] manifest not found: ${MANIFEST}`);
    process.exit(1);
  }
  const stems = JSON.parse(readFileSync(MANIFEST, "utf8"));
  if (!Array.isArray(stems) || stems.length === 0) {
    console.error("[skills-lite-presets] manifest.json must be a non-empty array of file stems");
    process.exit(1);
  }

  const entries = stems.map((stem, i) => {
    const file = join(PRESET_DIR, `${stem}.md`);
    if (!existsSync(file)) {
      console.error(`[skills-lite-presets] missing preset file: ${file}`);
      process.exit(1);
    }
    const parsed = parseSkillMarkdown(readFileSync(file, "utf8"));
    if (!parsed) {
      console.error(`[skills-lite-presets] could not derive a name from ${file}`);
      process.exit(1);
    }
    // First manifest entry is the default-enabled skill.
    return { ...parsed, enabled: i === 0 };
  });

  mkdirSync(OUT_DIR, { recursive: true });

  const banner = [
    "// AUTO-GENERATED by scripts/generate-skills-lite-presets.mjs — do not edit.",
    "// Source: skills-lite-presets/*.md + scripts/skills-lite-presets.manifest.json",
    "// The first manifest entry is the default-enabled preset (enabled: true).",
  ].join("\n");

  const body = `import type { SkillsLiteEntry } from "../stores/config";

export const PRESET_SKILLS_LITES: SkillsLiteEntry[] = ${JSON.stringify(entries, null, 2)};
`;

  writeFileSync(OUT, `${banner}\n\n${body}`, "utf8");
  console.log(`[skills-lite-presets] wrote ${OUT} (${entries.length} entries)`);
}

main();
