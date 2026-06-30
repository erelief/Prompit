# Built-in Prompts & Templates — Modification Guide

A reference for **humans and agents** on exactly where — and how — to change the
built-in prompt content in Prompit: the default Persona, the default Sparkle,
and the hardcoded system/meta prompts that wrap them.

> **Read this first.** Prompit has **three distinct layers** of prompt content,
> and each one is edited in a different place and behaves differently. Editing
> the wrong layer (or forgetting the Rust mirror) is the most common mistake.
> If you only read one section, read [§2 Architecture cheat-sheet](#2-architecture-cheat-sheet).

---

## 1. "I want to…" — quick lookup

| What you want to change | Layer | Section |
|---|---|---|
| The wording of the default **Translation Persona** ("Coding（编程）") | 1 — seed | [§3](#3-layer-1--built-in-persona-seed-translate-mode) |
| The wording of the default **Sparkle** ("Polish（润色）") | 1 — seed | [§4](#4-layer-2--built-in-sparkle-seed-sparkle-mode) |
| Ship **multiple** built-in personas/sparkles on first run | 1 — seed | [§3.3](#33-ship-more-than-one-builtin) / [§4.3](#43-ship-more-than-one-builtin) |
| Add a **new field** to personas/sparkles (e.g. `category`) | 1 — seed + schema | [§7.3](#73-add-a-new-field-to-personassparkles) |
| The fixed **"You are a translation engine…"** translate prompt | 3 — hardcoded | [§5.1](#51-translates-system-prompt) |
| The **"Output ONLY the transformed result"** guardrail suffix | 3 — hardcoded | [§5.2](#52-sparkles-guardrail-suffix) |
| The **"Optimize prompt"** / **"Summarize"** wand meta-prompts | 3 — hardcoded | [§5.3](#53-the-optimizeprompt--summarize-meta-prompts) |
| The **UI labels** for these editors (names, placeholders, tab titles) | — i18n | [§6](#6-i18n-labels) |
| Reset a user's built-ins back to defaults | — ops | [§7.5](#75-reset-a-users-builtins-to-defaults) |

---

## 2. Architecture cheat-sheet

The three layers, at a glance:

```
┌─────────────────────────────────────────────────────────────────────┐
│ Layer 3 — HARDCODED system & meta prompts  (src/services/llm-client.ts) │
│   buildSystemPrompt(), buildSparkleSystemPrompt(), optimizePrompt() │
│   → Apply to ALL users on next rebuild. Nothing persisted.          │
└─────────────────────────────────────────────────────────────────────┘
                              ▲ wraps
┌─────────────────────────────────────────────────────────────────────┐
│ Layer 1/2 — SEED constants  (src/stores/config.ts)                  │
│   DEFAULT_CODING_PERSONA  (persona seed)                            │
│   DEFAULT_POLISH_SPARKLE  (sparkle seed)                            │
│   → Written ONCE into encrypted personas.json / sparkles.json       │
│     on first run. After that, the user owns & can edit/delete them. │
└─────────────────────────────────────────────────────────────────────┘
                              ▲ persisted to
┌─────────────────────────────────────────────────────────────────────┐
│ Runtime storage — <data_dir>/personas.json, sparkles.json           │
│   AES-256-GCM encrypted via Rust crate::crypto.                     │
└─────────────────────────────────────────────────────────────────────┘
```

### The four things most people get wrong

1. **Built-ins are not flagged.** There is **no `isBuiltin` / `preset` / `system`
   field** on personas or sparkles. The "built-in" entry is just a hardcoded
   seed constant that gets written into the user's data file on first run.
   From that moment on it is **indistinguishable** from a user-created entry —
   the user can rename, edit, or delete it freely.

2. **Seed changes do NOT reach existing installs.** Seeding only happens when
   `personas.json` / `sparkles.json` is missing or empty (`config.ts:476`,
   `config.ts:729`). Editing a seed constant only affects **fresh installs**.
   To update an existing user you must either bump the schema with a migration
   or have them reset (see [§7.5](#75-reset-a-users-builtins-to-defaults)).

3. **Schema changes must mirror BOTH TypeScript and Rust.** A persona/sparkle
   is defined in `src/stores/config.ts` (frontend) **and** in
   `src-tauri/src/commands/{persona,sparkle}.rs` (backend, which (de)serializes
   the encrypted file). Add a field on one side and forget the other →
   deserialization failure or a silent drop.

4. **Rust serde needs `#[serde(default)]` for new fields.** Otherwise older
   persisted files (from users who installed before the field existed) will fail
   to load. See how `description` and `enabled` are handled in
   `src-tauri/src/commands/sparkle.rs:13-16`.

### File map (the canonical locations)

| Concern | File | Key lines |
|---|---|---|
| All TS interfaces + seed constants + load/save | `src/stores/config.ts` | schemas `:109-126`, persona seed `:466`, sparkle seed `:718`, `MODES` `:756` |
| Persona load/save (TS) | `src/stores/config.ts` | `loadPersonas :473`, `savePersonas :505` |
| Sparkle load/save (TS) | `src/stores/config.ts` | `loadSparkles :726`, `saveSparkles :746` |
| System/meta prompt construction | `src/services/llm-client.ts` | `optimizePrompt :282`, `buildSystemPrompt :355`, `buildSparkleSystemPrompt :370` |
| Persona persistence (Rust) | `src-tauri/src/commands/persona.rs` | struct `:9-15`, cmds `:47-55` |
| Sparkle persistence (Rust) | `src-tauri/src/commands/sparkle.rs` | struct `:9-17`, cmds `:49-57` |
| Command registration (Rust) | `src-tauri/src/lib.rs` | `:144-147` |
| Data-dir resolution | `src-tauri/src/lib.rs` | `get_data_dir :82` |
| Startup load | `src/main.ts` | `loadConfig :29`, `loadSparkles :34` |
| Settings UI (edit/add/delete) | `src/views/Settings.vue` | persona `:2194-2251`, sparkle `:2303-2388` |
| Runtime toolbar selection | `src/components/TranslateToolbar.vue` | sparkle dropdown `:267-330` |
| i18n labels | `src/locales/en.json`, `src/locales/zh-CN.json` | modes `en.json:25-28`, editor labels `en.json:102-118` |

> **Out of scope** (cross-linked for clarity): the provider/model list in
> `provider-presets.json` is a *separate*, read-only asset compiled into the
> binary via `include_str!` (`src-tauri/src/commands/presets.rs`). It is not a
> prompt template. The user dictionary (`DictEntry`, `config.ts:122`) is also
> adjacent but not prompt content.

---

## 3. Layer 1 — Built-in Persona seed (Translate mode)

A **persona** is a reusable style instruction injected into Translate mode
("translate like a software engineer", "translate formally", …). Exactly the
**enabled** persona(s) get appended to the translate system prompt.

### 3.1 Where it lives

The single built-in persona is a constant in `src/stores/config.ts:466`:

```ts
const DEFAULT_CODING_PERSONA: PersonaConfig = {
  name: "Coding（编程）",
  prompt:
    "You are a software developer with 10 years of professional experience in software engineering. You specialize in using precise, industry-standard professional software development terminology for technical communication, and your audience is cross-functional engineering teams, product managers, and technical stakeholders.",
  enabled: false,
};
```

It is seeded into the user's store on first run inside `loadPersonas()`
(`src/stores/config.ts:473-503`). The relevant branch is at `config.ts:498`:

```ts
// Nothing stored yet (fresh install): seed a reference preset the user
// can edit or delete. Mirrors the sparkle default-seeding behavior.
personaStore.personas = [DEFAULT_CODING_PERSONA];
await savePersonas();
```

### 3.2 Schema

`PersonaConfig` — `src/stores/config.ts:109-113` (frontend):

| Field | Type | Meaning |
|---|---|---|
| `name` | `string` | Display name (toolbar dropdown, card title, history tag). Must be unique — validated in Settings. |
| `prompt` | `string` | Style/role instruction. Injected into the translate system prompt as `- Additional style instructions: <prompt>`. |
| `enabled` | `boolean` | Whether this persona is active. Multiple personas may be enabled simultaneously (their prompts concatenate). |

Rust mirror — `PersonaEntry` in `src-tauri/src/commands/persona.rs:9-15`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaEntry {
    pub name: String,
    pub prompt: String,
    #[serde(default)]
    pub enabled: bool,
}
```

Persisted (encrypted) to `<data_dir>/personas.json` via
`crypto::encrypt("personas", …)` (`persona.rs:37-45`). Registered as Tauri
commands `read_personas` / `save_personas` in `src-tauri/src/lib.rs:144-145`.

### 3.3 How the persona prompt is consumed

`buildSystemPrompt()` in `src/services/llm-client.ts:355-368` filters for
**enabled** personas and appends each as an "Additional style instructions"
rule, then adds the fixed translate output rules:

```ts
function buildSystemPrompt(): string {
  const enabledPersonas = personaStore.personas.filter((p) => p.enabled);

  let rules = "";
  for (const persona of enabledPersonas) {
    rules += `\n- Additional style instructions: ${persona.prompt}`;
  }
  rules += "\n- Output ONLY the translated text, nothing else.";
  // … fixed output rules …
  return `You are a translation engine. Translate the user's input text to the target language.\nRules:${rules}\nTarget language: ${appConfig.target_lang}.`;
}
```

Note: personas have **no** `description` field (unlike sparkles).

### 3.4 Edit recipe — change the built-in persona

1. Edit the `name` / `prompt` of `DEFAULT_CODING_PERSONA` in
   `src/stores/config.ts:466-471`.
2. That's it for a simple text change. Existing installs are unaffected (see
   [§2, gotcha #2](#the-four-things-most-people-get-wrong)); only fresh installs
   get the new wording.
3. Build (`npm run tauri dev`) and verify on a **clean** profile (delete
   `<data_dir>/personas.json` first).

### 3.5 Ship more than one built-in

Replace the single-element seed array in `loadPersonas()` at `config.ts:498`:

```ts
personaStore.personas = [
  DEFAULT_CODING_PERSONA,
  DEFAULT_FORMAL_PERSONA,   // add new DEFAULT_* constants alongside
  DEFAULT_CASUAL_PERSONA,
];
await savePersonas();
```

---

## 4. Layer 2 — Built-in Sparkle seed (Sparkle mode)

A **sparkle** is a complete "transform whatever I type" system prompt used in
Sparkle mode (polish, summarize, rewrite formally, …). Exactly **one** sparkle
is enabled at a time (exclusive toggle).

### 4.1 Where it lives

The single built-in sparkle is a constant in `src/stores/config.ts:718`:

```ts
const DEFAULT_POLISH_SPARKLE: SparkleEntry = {
  name: "Polish（润色）",
  prompt:
    "Detect the language of the user's input. Adopt the role of a native speaker of that language. Rewrite the user's input as a more idiomatic, accurate, and natural expression in the same language, preserving the original meaning and intent.",
  description: "Polish the input like a native speaker of its language.",
  enabled: true,
};
```

Seeded on first run inside `loadSparkles()` (`src/stores/config.ts:726-744`),
with the seed branch at `config.ts:729-731`:

```ts
const entries = await invoke<SparkleEntry[]>("read_sparkles");
if (entries.length === 0) {
  sparkleStore.sparkles = [DEFAULT_POLISH_SPARKLE];
  await saveSparkles();
}
```

`loadSparkles()` is called once at startup from `src/main.ts:34`.

### 4.2 Schema

`SparkleEntry` — `src/stores/config.ts:115-120` (frontend):

| Field | Type | Meaning |
|---|---|---|
| `name` | `string` | Display name (toolbar dropdown, card title, history tag). Must be unique. |
| `prompt` | `string` | The system prompt sent to the LLM when this sparkle is active. |
| `description` | `string` | Short human description. Shown under the name in the card **and** used as the floating input placeholder (`src/views/FloatingInput.vue:122-128`). |
| `enabled` | `boolean` | Exclusive selection flag — only one sparkle may be enabled (`toggleSparkle`, `Settings.vue:572-581`). Cannot be unset if it's the last sparkle. |

Rust mirror — `SparkleEntry` in `src-tauri/src/commands/sparkle.rs:9-17`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparkleEntry {
    pub name: String,
    pub prompt: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub enabled: bool,
}
```

Persisted (encrypted) to `<data_dir>/sparkles.json` via
`crypto::encrypt("sparkles", …)` (`sparkle.rs:39-47`). Registered as Tauri
commands `read_sparkles` / `save_sparkles` in `src-tauri/src/lib.rs:146-147`.

> The `#[serde(default)]` on `description` and `enabled` is **intentional
> backward compatibility** — see the legacy-migration test at
> `sparkle.rs:79-88`. Any new field you add should follow the same pattern, or
> older persisted files will fail to deserialize.

### 4.3 How the sparkle prompt is consumed

`buildSparkleSystemPrompt()` in `src/services/llm-client.ts:370-379` finds the
first **enabled** sparkle and appends a fixed guardrail suffix (see
[§5.2](#52-sparkles-guardrail-suffix)):

```ts
function buildSparkleSystemPrompt(): string {
  const enabled = sparkleStore.sparkles.find((s) => s.enabled);
  if (!enabled) {
    return "You are a helpful assistant. Output ONLY the result, nothing else.";
  }
  return (
    enabled.prompt +
    "\n\nIMPORTANT: Output ONLY the transformed result. Do not include any explanations, notes, meta-commentary, or original text. Output just the result."
  );
}
```

### 4.4 Edit recipe — change the built-in sparkle

1. Edit `name` / `prompt` / `description` of `DEFAULT_POLISH_SPARKLE` in
   `src/stores/config.ts:718-724`.
2. For a text-only change, nothing else is needed. Verify on a clean profile
   (delete `<data_dir>/sparkles.json`).

### 4.5 Ship more than one built-in

Replace the seed array in `loadSparkles()` at `config.ts:730`:

```ts
sparkleStore.sparkles = [
  DEFAULT_POLISH_SPARKLE,
  DEFAULT_SUMMARIZE_SPARKLE,   // add new DEFAULT_* constants; keep exactly one enabled:true
];
await saveSparkles();
```

---

## 5. Layer 3 — Hardcoded system & meta prompts

These live **directly in code** (`src/services/llm-client.ts`), are never
persisted, and apply to **every** user immediately on the next rebuild — no
migration, no seeding involved.

### 5.1 Translate's system prompt

`buildSystemPrompt()` — `src/services/llm-client.ts:355-368`.

The fixed scaffold is `You are a translation engine. Translate the user's input
text to the target language.` followed by hardcoded `Rules:`:

- `- Output ONLY the translated text, nothing else.`
- `- Preserve the original punctuation style and line breaks.`
- `- Do not add explanations, notes, or any extra content.`
- `- If the input is already in the target language, output it as-is.`

…then each enabled persona's prompt as `- Additional style instructions: …`,
and finally `Target language: <appConfig.target_lang>`.

**To change** the fixed translate behavior, edit this function. (Persona
content itself is user data — see [§3](#3-layer-1--built-in-persona-seed-translate-mode).)

### 5.2 Sparkle's guardrail suffix

`buildSparkleSystemPrompt()` — `src/services/llm-client.ts:370-379`.

The sparkle's user prompt is concatenated with this fixed suffix:

```
\n\nIMPORTANT: Output ONLY the transformed result. Do not include any explanations, notes, meta-commentary, or original text. Output just the result.
```

Also a hardcoded fallback when no sparkle is enabled:
`"You are a helpful assistant. Output ONLY the result, nothing else."`

**To change** the output-discipline wording, edit these two strings.

### 5.3 The `optimizePrompt` / summarize meta-prompts

`optimizePrompt(rawPrompt, mode)` — `src/services/llm-client.ts:282-353`. This
powers the **"Optimize prompt"** and **"Summarize"** wand buttons in Settings
(`Settings.vue:542` for personas, `:736` sparkle optimize, `:759` summarize).
It contains **three distinct built-in system prompts**, selected by `mode`:

| `mode` value | Triggered by | System prompt content | Lines |
|---|---|---|---|
| `"sparkle"` | Sparkle "Optimize" wand | Prompt organizer: restructure the user's prompt without changing intent. | `:294` |
| `"summarize"` | Sparkle "Summarize" wand (`Settings.vue:759`) | Description writer: detect language, produce a `<verb> the input into <result>` line under 20 chars (ZH) / 12 words (EN). | `:296` |
| default (`"translate"` or omitted) | Persona "Optimize" wand (`Settings.vue:542`) | Persona optimizer: converts a vague style hint into a `You are a [role] with [years] of experience…` template, in the input's language, with EN+ZH examples. | `:297-308` |

**To change** how the wand buttons rewrite prompts, edit the relevant branch.
Note the default branch contains worked examples in both English and Chinese —
update both if you change the template.

---

## 6. i18n labels

The editor UI is fully localized. Labels live in `src/locales/en.json` and
`src/locales/zh-CN.json` (keep them in sync; the build runs
`scripts/validate-i18n.mjs`).

Key locations in `src/locales/en.json`:

- Mode names: `modes.translate` / `modes.sparkle` — `en.json:25-28`.
- Settings labels: `settings.translationPersona`, `settings.noPersonasYet`,
  `settings.personaName`, `settings.sparkleTitle`, `settings.noSparklesYet`,
  `settings.sparkleName`, `settings.sparklePrompt`, `settings.sparkleDescription`,
  `settings.sparkleModel`, … — `en.json:102-118`.
- Toolbar/history: `settings.selectSparkle` (`:154`), `settings.enablePersona` /
  `disablePersona` (`:144-145`), history tag `history.persona` (`:198`).

**To add a new label**, add a key to **both** `en.json` and `zh-CN.json` (the
validator enforces parity), then reference it as `t('settings.yourKey')` in the
Vue component. Filenames and `name`/`description` *values* inside seed constants
(§3.1, §4.1) are **not** i18n keys — they are literal strings shown verbatim,
so bilingual seed names use the `Name（中文名）` convention.

---

## 7. Common workflows

### 7.1 Change the wording of the default Polish sparkle

Edit `DEFAULT_POLISH_SPARKLE` at `src/stores/config.ts:718-724`. Done — only
fresh installs are affected; existing users keep their copy.

### 7.2 Change the "output only the result" guardrail

Edit the suffix string in `buildSparkleSystemPrompt()` at
`src/services/llm-client.ts:377`. Applies to **all** users on next rebuild —
no migration.

### 7.3 Add a new field to personas/sparkles

Full checklist (sparkle shown; persona is analogous):

1. **TS interface** — add the field to `SparkleEntry` in
   `src/stores/config.ts:115-120`.
2. **Rust struct** — add the field to `SparkleEntry` in
   `src-tauri/src/commands/sparkle.rs:9-17` **with `#[serde(default)]`** (so
   old `sparkles.json` files still load).
3. **Seed constant** — set the field on `DEFAULT_POLISH_SPARKLE`
   (`config.ts:718`) and in the seed array (`loadSparkles`, `config.ts:730`).
4. **Load normalization** — if needed, default-fill the field in the
   `entries.map(...)` branch of `loadSparkles()` (`config.ts:735-738`), the way
   `description` is normalized.
5. **UI** — add an input for the field in `Settings.vue` (sparkle editor
   `:2303-2388`) and/or `EditableCardList.vue`; update `validateSparkle`
   (`Settings.vue:560`) if it should be required.
6. **New-sparkle draft** — initialize the field in the draft object at
   `Settings.vue:2315`.
7. **Auto-persist** — already covered by the watchers at
   `Settings.vue:998-1006`; no change needed unless you change the store shape.
8. **Backward-compat test** — mirror the legacy-migration test at
   `sparkle.rs:79-88` for the new field.

### 7.4 Reset a user's built-ins to defaults

There is **no in-app "reset to defaults" button**. To reset, delete the
encrypted data file(s) in the app data dir (resolved by `get_data_dir` at
`src-tauri/src/lib.rs:82` → Tauri `app_config_dir()`):

- `personas.json` — personas
- `sparkles.json` — sparkles

On next launch, `loadPersonas()` / `loadSparkles()` see an empty/missing file
and re-seed from `DEFAULT_CODING_PERSONA` / `DEFAULT_POLISH_SPARKLE`.

### 7.5 Add a true read-only built-in (does NOT exist yet)

The current design has **no concept of immutable, non-deletable, reset-able
built-in templates**. If you need that, you would have to:

1. Add a discriminator field (e.g. `is_builtin: boolean`) to
   `PersonaConfig` / `SparkleEntry` and the Rust mirrors.
2. Gate the remove/edit UI for built-in entries in `EditableCardList.vue` and
   `Settings.vue`.
3. Add a "reset to defaults" flow that re-injects the seed constants while
   preserving user-created entries.

Until that exists, treat "built-in" as "the seed the user received on first
run, which they are free to change."

---

## 8. Gotchas & current limitations

- **No `isBuiltin` flag.** Built-ins are user-editable and deletable. See [§2](#2-architecture-cheat-sheet).
- **No "reset to defaults" UI.** Manual file deletion only — [§7.4](#74-reset-a-users-built-ins-to-defaults).
- **Seed edits don't reach existing installs.** Only fresh installs (empty
  data file) get re-seeded. Plan a migration or document the reset step.
- **Persona migration history.** Personas used to live inside `config.json`;
  `loadPersonas()` still migrates them out on first run (`config.ts:480-495`)
  and strips them from `config.json`. Don't re-add personas to `config.json`.
- **i18n parity is enforced.** `npm run build` runs `scripts/validate-i18n.mjs`;
  any new label must exist in both `en.json` and `zh-CN.json`.
- **Provider/model presets are a different system.** `provider-presets.json`
  is a read-only asset compiled into the binary (`src-tauri/src/commands/presets.rs`),
  never written by the app — do not confuse it with prompt templates.
- **Two test files worth keeping green** after any schema change:
  `src-tauri/src/commands/persona.rs` (`:57-82`) and
  `src-tauri/src/commands/sparkle.rs` (`:59-89`) — they guard the
  round-trip and legacy-migration behavior.
