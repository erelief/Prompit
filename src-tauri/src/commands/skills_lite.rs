use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::crypto::{self, EncryptedPayload};

/// Crypto scope for skills-lite data. Feeds AES-GCM key derivation.
const SCOPE: &str = "skills_lite";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsLiteEntry {
    pub name: String,
    pub prompt: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub enabled: bool,
}

fn skills_lites_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::get_data_dir(app)?;
    fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;
    Ok(dir.join("skills_lite.json"))
}

fn load_skills_lites_encrypted(app: &AppHandle) -> Result<Vec<SkillsLiteEntry>, String> {
    let path = skills_lites_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("read: {e}"))?;
    let payload: EncryptedPayload =
        serde_json::from_str(&content).map_err(|e| format!("parse: {e}"))?;
    let bytes = crypto::decrypt(SCOPE, &payload)?;
    serde_json::from_slice(&bytes).map_err(|e| format!("deserialize: {e}"))
}

fn save_skills_lites_encrypted(app: &AppHandle, skills_lites: &[SkillsLiteEntry]) -> Result<(), String> {
    let path = skills_lites_path(app)?;
    let json = serde_json::to_vec(skills_lites).map_err(|e| format!("serialize: {e}"))?;

    let payload = crypto::encrypt(SCOPE, &json)?;
    let out = serde_json::to_string_pretty(&payload).map_err(|e| format!("serialize enc: {e}"))?;
    fs::write(&path, out).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn read_skills_lites(app: AppHandle) -> Result<Vec<SkillsLiteEntry>, String> {
    load_skills_lites_encrypted(&app)
}

#[tauri::command]
pub fn save_skills_lites(app: AppHandle, skills_lites: Vec<SkillsLiteEntry>) -> Result<(), String> {
    save_skills_lites_encrypted(&app, &skills_lites)
}

/// Escape a string for a single-quoted YAML scalar: wrap in single quotes and
/// double up any embedded single quotes. Example: `it's me` -> `'it''s me'`.
fn yaml_single_quote(s: &str) -> String {
    let escaped = s.replace('\'', "''");
    format!("'{escaped}'")
}

/// Serialize a single skill entry to the docs/SKILL.md plaintext template:
///
/// ```markdown
/// ---
/// name: {name}
/// description: '{description}'
/// ---
///
/// # {name}
/// {prompt}
/// ```
fn format_skill_markdown(entry: &SkillsLiteEntry) -> String {
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("name: {}\n", yaml_single_quote(&entry.name)));
    out.push_str(&format!(
        "description: {}\n",
        yaml_single_quote(&entry.description)
    ));
    out.push_str("---\n\n");
    out.push_str(&format!("# {}\n", entry.name));
    // Preserve the prompt verbatim (trailing newline optional).
    let prompt = entry.prompt.trim_end_matches('\n');
    out.push_str(prompt);
    if !prompt.is_empty() {
        out.push('\n');
    }
    out
}

/// Parse the docs/SKILL.md plaintext template back into a skill entry.
///
/// Accepts:
/// - An optional YAML frontmatter block delimited by leading `---` lines,
///   containing `name:` and `description:` fields.
/// - A body whose first `# ` heading sets the name (when frontmatter is absent
///   or omits `name:`) and the text after that heading becomes the prompt.
///
/// Returns `None` when no name can be derived.
fn parse_skill_markdown(raw: &str) -> Option<SkillsLiteEntry> {
    // Strip UTF-8 BOM if present.
    let content = raw.strip_prefix('\u{feff}').unwrap_or(raw);
    let lines: Vec<&str> = content.lines().collect();

    let mut name: Option<String> = None;
    let mut description: Option<String> = None;
    let mut body_start: usize = 0;

    // Frontmatter: first line is exactly "---", find the matching closer.
    if lines.first().map(|l| l.trim_end()) == Some("---") {
        for (i, line) in lines.iter().enumerate().skip(1) {
            if line.trim_end() == "---" {
                // Parse the frontmatter lines [1..i) as `key: value`.
                for fl in &lines[1..i] {
                    if let Some(v) = parse_yaml_kv(fl, "name") {
                        name = Some(v);
                    } else if let Some(v) = parse_yaml_kv(fl, "description") {
                        description = Some(v);
                    }
                }
                body_start = i + 1;
                break;
            }
        }
    }

    // Body: skip blank lines, then find the first `# ` heading.
    let body_lines: Vec<&str> = lines[body_start..]
        .iter()
        .skip_while(|l| l.trim().is_empty())
        .copied()
        .collect();

    let mut prompt: String = String::new();
    let mut heading_name: Option<String> = None;
    for (i, line) in body_lines.iter().enumerate() {
        if let Some(rest) = line.trim_start().strip_prefix("# ") {
            heading_name = Some(rest.trim().to_string());
            // Prompt is everything after the heading line, blank lines trimmed.
            prompt = body_lines[i + 1..]
                .iter()
                .copied()
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string();
            break;
        }
    }

    let final_name = name.or(heading_name)?.trim().to_string();
    if final_name.is_empty() {
        return None;
    }

    Some(SkillsLiteEntry {
        name: final_name,
        description: description.unwrap_or_default().trim().to_string(),
        prompt,
        enabled: false,
    })
}

/// Parse a `key: value` YAML line, returning the unquoted value if `key` matches.
fn parse_yaml_kv(line: &str, key: &str) -> Option<String> {
    let trimmed = line.trim();
    let prefix = format!("{key}:");
    let rest = trimmed.strip_prefix(&prefix)?;
    let v = rest.trim();
    // Strip matching surrounding quotes (single or double). For single-quoted
    // YAML scalars, also collapse doubled single quotes back to one.
    if v.len() >= 2 && v.starts_with('\'') && v.ends_with('\'') {
        Some(v[1..v.len() - 1].replace("''", "'").trim().to_string())
    } else if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
        Some(v[1..v.len() - 1].trim().to_string())
    } else {
        Some(v.to_string())
    }
}

#[tauri::command]
pub fn export_skills_lite_markdown(
    _app: AppHandle,
    file_path: String,
    entry: SkillsLiteEntry,
) -> Result<(), String> {
    let markdown = format_skill_markdown(&entry);
    fs::write(&file_path, markdown).map_err(|e| format!("write file: {e}"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsLiteImportResult {
    pub imported: usize,
    pub skipped: usize,
}

#[tauri::command]
pub fn import_skills_lite_markdown(
    app: AppHandle,
    file_paths: Vec<String>,
) -> Result<SkillsLiteImportResult, String> {
    let mut entries = load_skills_lites_encrypted(&app)?;
    let mut imported = 0usize;
    let mut skipped = 0usize;

    for path in &file_paths {
        let raw = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        match parse_skill_markdown(&raw) {
            Some(entry) => {
                entries.push(entry);
                imported += 1;
            }
            None => skipped += 1,
        }
    }

    if imported > 0 {
        save_skills_lites_encrypted(&app, &entries)?;
    }

    Ok(SkillsLiteImportResult { imported, skipped })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skills_lite_entry_serialize_roundtrip() {
        let entries = vec![SkillsLiteEntry {
            name: "Formal".to_string(),
            prompt: "Translate formally".to_string(),
            description: "Rewrite input formally".to_string(),
            enabled: true,
        }];
        let json = serde_json::to_string(&entries).unwrap();
        let deserialized: Vec<SkillsLiteEntry> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.len(), 1);
        assert_eq!(deserialized[0].name, "Formal");
        assert_eq!(deserialized[0].description, "Rewrite input formally");
        assert!(deserialized[0].enabled);
    }

    #[test]
    fn test_legacy_skills_lite_without_description_defaults_empty() {
        // Older files persisted before the `description` field existed.
        let json = r#"[{"name":"Polish","prompt":"Rewrite nicely","enabled":true}]"#;
        let deserialized: Vec<SkillsLiteEntry> = serde_json::from_str(json).unwrap();
        assert_eq!(deserialized.len(), 1);
        assert_eq!(deserialized[0].name, "Polish");
        assert_eq!(deserialized[0].description, "");
        assert!(deserialized[0].enabled);
    }

    #[test]
    fn test_markdown_roundtrip_basic() {
        let entry = SkillsLiteEntry {
            name: "Polish".to_string(),
            prompt: "Rewrite the input text in a polished tone.".to_string(),
            description: "Rewrite input formally".to_string(),
            enabled: true,
        };
        let md = format_skill_markdown(&entry);
        let parsed = parse_skill_markdown(&md).expect("roundtrip should parse");
        assert_eq!(parsed.name, "Polish");
        assert_eq!(parsed.description, "Rewrite input formally");
        assert_eq!(parsed.prompt, "Rewrite the input text in a polished tone.");
        assert!(!parsed.enabled); // import always sets enabled = false
    }

    #[test]
    fn test_markdown_roundtrip_multiline_prompt() {
        let entry = SkillsLiteEntry {
            name: "Translator".to_string(),
            prompt: "Line one.\nLine two.\n\nLine four.".to_string(),
            description: "".to_string(),
            enabled: false,
        };
        let md = format_skill_markdown(&entry);
        let parsed = parse_skill_markdown(&md).expect("roundtrip should parse");
        assert_eq!(parsed.name, "Translator");
        assert_eq!(parsed.description, "");
        assert_eq!(parsed.prompt, "Line one.\nLine two.\n\nLine four.");
    }

    #[test]
    fn test_markdown_roundtrip_quote_in_description() {
        let entry = SkillsLiteEntry {
            name: "Quirky".to_string(),
            prompt: "do thing".to_string(),
            description: "it's a \"quoted\" description".to_string(),
            enabled: false,
        };
        let md = format_skill_markdown(&entry);
        let parsed = parse_skill_markdown(&md).expect("roundtrip should parse");
        assert_eq!(parsed.name, "Quirky");
        assert_eq!(parsed.description, "it's a \"quoted\" description");
    }

    #[test]
    fn test_parse_bom_prefixed() {
        let md = "\u{feff}---\nname: 'Bom'\ndescription: ''\n---\n\n# Bom\nhello";
        let parsed = parse_skill_markdown(md).expect("BOM should parse");
        assert_eq!(parsed.name, "Bom");
        assert_eq!(parsed.prompt, "hello");
    }

    #[test]
    fn test_parse_unquoted_description() {
        // Tolerate unquoted YAML scalars (common in hand-written files).
        let md = "---\nname: Plain\ndescription: A plain value\n---\n\n# Plain\nbody";
        let parsed = parse_skill_markdown(md).expect("unquoted should parse");
        assert_eq!(parsed.name, "Plain");
        assert_eq!(parsed.description, "A plain value");
        assert_eq!(parsed.prompt, "body");
    }

    #[test]
    fn test_parse_no_frontmatter_uses_heading() {
        // A plain markdown file with no frontmatter: derive name from heading.
        let md = "# From Heading\n\nThe prompt body.";
        let parsed = parse_skill_markdown(md).expect("heading should supply name");
        assert_eq!(parsed.name, "From Heading");
        assert_eq!(parsed.description, "");
        assert_eq!(parsed.prompt, "The prompt body.");
    }

    #[test]
    fn test_parse_no_name_returns_none() {
        // No frontmatter and no `# ` heading: cannot derive a name.
        assert!(parse_skill_markdown("just some text\nwithout a heading").is_none());
        assert!(parse_skill_markdown("").is_none());
    }
}
