<div align="center">

English | **简体中文**

</div>

<p align="center">
  <img src="public/prompit_logo.svg" width="120" alt="Prompit">
</p>

<h1 align="center">Prompit</h1>

<p align="center">
  A small AI helper that shows up when called, and leaves when dismissed.
</p>

---

> *Prompit* is a portmanteau of *Prompt it*.
>
> And that really is all it does.

Prompit is a small floating window that lives in the background. **Press a shortcut when you need it**, it appears next to your cursor, uses an AI model to do a few simple things — translate, polish, answer a question — then drops the result right back where your cursor is and disappears on its own.

## Highlights

### 🪂 Summon on demand, dismiss in a blink
Press `Alt+Y` (customizable) and the window pops up next to your cursor. After typing your request, press `Enter`, and after the result arrives press `Enter` again — the result is pasted straight back to where you were, and the window disappears on its own.

No window switching, no copy-paste.

### 🌐 Translation
Translates your input into a target language, with several customization options:
- **Multiple target languages** — you can add your own and reorder them. I've tried it; even Classical Chinese works.
- **Translation persona**: set the "translator's" identity to better match your translation scenario.
- **User dictionary**: a dictionary you maintain yourself, to pin specific translations for certain words.

### ✨ Skills Lite
Beyond translation, you can load commonly-used system prompts as preset templates into the app — effectively a single-file skill system that speaks SKILL.md (and yes, it really supports direct import).
- Built-in quick-question skill: AI replies to your question with a concise answer.
- Built-in XML-tag-prompt skill: structures your prompt with XML tags for better organization; AI fills in missing information automatically.
- Built-in polish skill: detects the language of your input and polishes it as if a native speaker wrote it.
- Supports importing standard-format SKILL.md files, and exporting skills in the same format.
- Supports connecting a third-party service for AI search, so it can pull real-time information from the web and keep the sources.

### 🔌 AI
Prompit's features run on AI models, **but it doesn't ship with one** — you'll need to connect a third-party AI service (any OpenAI-API-compatible provider; mainstream platforms come as presets in the app).

- **Everything Prompit does today is free** — no paid features, no in-app purchases, no subscriptions. Any cost is between you and the third-party service. If you use a self-hosted model, it's fully private and costs nothing.
- **A cheap model is plenty** — the app's tasks are very simple; any modern mainstream AI model handles them easily. Cheap and fast matter more than flagship.

### 🔧 Other details
- Basic keyboard support, designed not to interrupt a continuous typing flow.
- History, so nothing is lost to a slip of the hand:
  - Searchable history records.
  - When web search is enabled, history entries keep their sources for traceability.


## With so many AI input methods out there, why build this?

Prompit is **not** an AI input method. It's closer to a launcher like [Alfred](https://www.alfredapp.com/) or [Listary](https://www.listary.com/).

Modern AI input methods tend to emphasize "voice input" efficiency. But even when I'm alone, talking to my computer still feels deeply awkward to me. AI input methods also lack a sense of boundary — I only want AI to show up *when I actually want it*. So Prompit and input methods don't really compete for the same niche.

The original idea came from [sxzxs/Real-time-translation-typing](https://github.com/sxzxs/Real-time-translation-typing), a real-time translation tool built on an [AHK v2 script](https://www.autohotkey.com/v2/). I first forked it into a version powered by an AI service — [erelief/Real-time-translation-typing-LLM](https://github.com/erelief/Real-time-translation-typing-LLM) — but AHK had too many limitations, was hard to pick up, and couldn't go cross-platform. So I built this.

## Security notes
This project is entirely the product of Vibe Coding. I can't really call myself a programmer, but I've understood and learned as much as I reasonably could. The app has some basic security in place (all personal data is stored encrypted rather than in plain text, with simple data import/export and destruction) — but please still keep the following in mind:
- The software may have rough edges.
- Use official provider endpoints; be wary of third-party relay/proxy services.
- **Do not feed privacy-sensitive, important, or high-risk information** to the AI service.
- Use a **dedicated** API key for this app, and rotate it regularly.
- Clear your history from time to time.


## Demo

<!-- TODO: add screenshots here -->
<!-- Suggested paths: docs/screenshots/summon.png (the popup), docs/screenshots/translate.png (translation), docs/screenshots/skills.png (skills) -->
<!-- Example: <p align="center"><img src="docs/screenshots/summon.png" width="600" alt="Summoning the window"></p> -->

(Screenshots coming soon.)

## Download & install

Grab the installer for your system from the [Releases page](https://github.com/erelief/Prompit/releases):

- **Windows**: `.msi` or `.exe`
- **macOS**: `.dmg`
- **Linux**: `.deb` or `.AppImage`

After installing and opening it, the first run will ask you to connect a model provider — without one, it's just an empty shell. Connect one and you're ready.


## FAQ

**Does it snoop on what I type?**

No. It only moves when you call it.

**Can it only translate?**

Translation is its main feature — and the original motivation for the project — but not the only one. Any time you think "I want a quick hit of AI here, but I don't want to switch to a browser window," it fits. Define your own with Skills Lite.

**Can it run a sequence of tasks for me (like "search → summarize → email")?**

No. It returns one response per request. It won't plan ahead or chain steps together on its own.

**Is it trying to replace my input method?**

No. Your input method is yours; Prompit is Prompit. Keep typing with your system keyboard, and summon it only when you need it.

**Where's my data stored?**

On your machine, encrypted. Nothing gets uploaded to a Prompit server (there isn't one). Encrypted import/export is supported too.

**Do I have to pay for a model?**

That depends on your chosen provider and model. Many providers offer free quotas, and self-hosted models are supported too.

## For developers

<details>
<summary>Tech stack & local development</summary>

Prompit is built with Tauri 2 + Vue 3 + TypeScript, cross-platform (Windows / macOS / Linux).

```bash
# Install dependencies
npm install

# Run in development
npm run tauri dev

# Build
npm run tauri build
```

Requires Node.js and a Rust toolchain.

</details>

## License

[Apache License 2.0](./LICENSE)
