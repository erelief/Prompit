<div align="center">

English | **简体中文**

</div>

<p align="center">
  <img src="public/prompit_logo.svg" width="120" alt="Prompit">
</p>

<h1 align="center">Prompit</h1>

<p align="center">
  A tiny translator that shows up when you call it.<br>
  Press a shortcut, it appears. One sentence in, one sentence out, and it's gone.
</p>

---

> The name is a mashup of *Prompt it* — as in, "give it a prompt."
> And that really is all it knows how to do.

Prompit is a small floating window that lives in the background. **Press a shortcut when you need it**, it pops up near your cursor, uses an AI model to translate a sentence (or tweak one for you), then drops the result right back at your cursor and disappears.

It only does small jobs — and it doesn't ship with a model of its own. **You'll need to connect a third-party AI service for it to do anything** (see the "Powered by AI" section below).

## What it is / What it isn't

Let's set expectations straight:

| It is ✅ | It isn't ❌ |
|---|---|
| A floating window that appears on a shortcut | An always-on, auto-popping assistant |
| A "dumb" app that returns one sentence per request | An AI agent that thinks, plans, and chains tasks together |
| A small tool: translation first, light skills second | An AI input method that takes over your typing |
| Your input method stays yours; translation stays separate | It won't replace or intercept your system keyboard |

In short: **give it a sentence, get a sentence back. It won't take a single extra step on its own.**

### Why not an AI agent?

An AI agent browses the web, decides its next move, and runs through a whole sequence of tasks by itself. Prompit doesn't.

Every request does exactly one thing — it takes the text you gave it and reshapes it the way you asked (translate, polish, …), then drops the result right back where your cursor was. Done. Gone. It doesn't ponder life, and it won't ponder yours for you either.

### Why not an AI input method?

An AI input method hangs around wherever you type and thinks about every keystroke. A lot of people don't want that kind of constant company.

Prompit goes the other way:

- **It basically doesn't exist most of the time** — don't press the shortcut and it sits quietly in the background. It doesn't get in your way or watch you type.
- **Translation and input method stay out of each other's business** — keep using your system keyboard; only summon Prompit when you actually need to translate.
- **It comes when called, and leaves completely when dismissed** — hit `Esc` when you're done and it's gone.
- **Way more customizable than an input method** — shortcuts, target languages, translation style, dictionaries, light skills, all yours to change (see [Customization](#customization)).

## Highlights

### 🪂 Summon on demand, dismiss in a blink
Press `Alt+Y` (customizable) and the window pops up near your cursor. Hit `Enter` and the result is pasted straight back to where you were typing — the window vanishes on its own. No window switching, no copy-paste.

### 🌐 Translation first
- Multiple target languages — and you can add your own, reorder them too.
- **Personas**: want the translation to sound like a programmer wrote it, like a formal document, or like a chat? Just add a persona.
- **User dictionary**: got words you want translated a specific way (names, jargon)? Drop them in the dictionary and it'll listen every time.

### ✨ Light skills, second
Beyond translation, it can do other one-sentence transformations — like **Polish**, which rewrites your text in the *same* language to read more naturally.

We call these little jobs "skills." You can write and edit your own, and import/export them as Markdown files — take them with you when you switch machines.

### 🔌 Powered by AI — but you pick the model
Prompit's features run on AI models, **but it doesn't ship with one** — you'll need to connect a third-party AI service (say, OpenAI, DeepSeek, Kimi, Zhipu, Alibaba Cloud, … 20+ common ones come as presets). Connect one, and you're set.

Why no built-in model? Because models cost money, and nobody's in a good position to foot that bill for you. So you pick the model, you handle the bill — Prompit just passes your text over and brings the result back.

**Two things to be clear about, money-wise:**

- **Prompit itself is currently free** — no paid features, no in-app purchases, no subscriptions. It's open source (Apache-2.0); just use it.
- **The AI service you connect may charge** — that depends on your chosen provider and model. Many offer free quotas, and local models work too. Pick a **cheap, lightweight** one — Prompit only does small jobs, no need to overpay for this.

> Your wallet will thank you.

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

## Quick start

1. **Install and open**, then follow the first-run setup to connect a model provider (pick a cheap one).
2. **Close the setup window.**
3. **Press `Alt + Y`** — the window pops up. Type what you want translated, hit `Enter`, and the result lands right at your cursor.

That's it. Three steps.

> 💡 Not sure about the shortcut on day one? A little hint pops up at launch to tell you. Forget it later? It's always there (and editable) in Settings.

## Customization

Don't like the defaults? Almost everything's tweakable:

- **Shortcuts**: summon (`Alt+Y`), switch mode (`Alt+M`), and more — all re-recordable.
- **Appearance**: light / dark / follow system, opacity, font size.
- **Target languages**: common ones built in, plus your own — reorder freely.
- **Translation personas**: define your own translation styles.
- **User dictionary**: pin specific translations, with import/export.
- **Light skills**: write, edit, import/export as Markdown.
- **Launch on startup**, **close after sending**, **history limit**, …

## FAQ

**Does it snoop on what I type?**
No. Until you press the shortcut, it just sits in the background — it doesn't touch your keyboard or hijack your input method. It only moves when you call it.

**Can it run a sequence of tasks for me (like "search → summarize → email")?**
No. It returns one sentence per request. It won't plan ahead or chain steps together on its own. For that, go find an agent — not this.

**Is it trying to replace my input method?**
No. Your input method is yours; translation is translation's business. Keep typing with your system keyboard, and summon Prompit only when you need to translate.

**Where's my data stored?**
On your machine, encrypted. Nothing gets uploaded to a Prompit server (there isn't one). You can also export an encrypted backup and restore it on a new machine.

**Do I have to pay for a model?**
Prompit itself is free, but the AI service you connect may charge — that depends on your chosen provider and model. Many offer free quotas, and local models work too. A cheap lightweight model is more than enough.

## For developers

<details>
<summary>Tech stack & local development (click to expand)</summary>

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
