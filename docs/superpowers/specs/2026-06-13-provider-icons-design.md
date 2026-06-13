# Provider Icons Design

Add monochrome provider brand icons across the app, sourced from @lobehub/icons SVGs, bundled locally for full offline use.

## 1. File Structure

```
src/components/icons/providers/
├── index.ts                # PROVIDER_ICONS map + re-exports
├── ProviderIcon.vue        # Public component (lookup + Cloud fallback)
├── OpenAI.vue
├── DeepSeek.vue
├── Kimi.vue
├── Zhipu.vue
├── Minimax.vue
├── Ollama.vue
├── README.md               # Developer guide for adding new icons
```

## 2. Icon Components

Each icon is a minimal Vue SFC rendering a raw SVG with `currentColor`:

```vue
<template>
  <svg :width="size" :height="size" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
    <path d="..." fill="currentColor" />
  </svg>
</template>

<script setup lang="ts">
defineProps<{ size?: number }>();
</script>
```

- All `fill` / `stroke` attributes use `currentColor`
- Color is controlled by parent CSS `color`, adapting to light/dark theme automatically
- Each accepts a `size` prop (default 16)

## 3. Icon Map (`index.ts`)

```ts
import type { Component } from 'vue'
import OpenAI from './OpenAI.vue'
import DeepSeek from './DeepSeek.vue'
import Kimi from './Kimi.vue'
import Zhipu from './Zhipu.vue'
import Minimax from './Minimax.vue'
import Ollama from './Ollama.vue'

export const PROVIDER_ICONS: Record<string, Component> = {
  openai: OpenAI,
  deepseek: DeepSeek,
  kimi: Kimi,
  zhipu: Zhipu,
  minimax: Minimax,
  ollama: Ollama,
}
```

Key convention: lowercase (e.g. `openai`, `deepseek`).

## 4. ProviderIcon Component

Public API consumed everywhere:

```ts
Props: {
  icon: string    // map key from provider-presets.json icon field
  size?: number   // default 16
}
```

Rendering logic:
1. `PROVIDER_ICONS[icon]` exists → render that component
2. Otherwise → render Lucide `Cloud` icon as fallback

Applies to: Custom providers, missing icons, empty `icon` field — all show the same Cloud fallback.

## 5. Data Layer Changes

### provider-presets.json

Add `icon` field to each preset:

```json
{
  "name": "OpenAI",
  "provider_name": "OpenAI",
  "icon": "openai",
  "base_url": "https://api.openai.com/v1",
  "api_url": "https://platform.openai.com/apikeys",
  "api_format": { ... }
},
{
  "name": "Custom",
  "provider_name": "",
  "icon": "",
  "base_url": "",
  "api_url": "",
  "api_format": {}
}
```

### TypeScript Interfaces

- `ProviderPreset` — add `icon: string`
- `ProviderConfig` — unchanged (icon resolved via `preset` field at runtime)

### Rust Struct

- `ProviderPreset` — add `icon: String` with `#[serde(default)]`

### Icon Lookup Function

```ts
function getProviderIcon(provider: ProviderConfig, presets: ProviderPreset[]): string {
  if (!provider.preset) return ''
  return presets.find(p => p.name === provider.preset)?.icon ?? ''
}
```

No name matching, no guessing. Only bound to `provider-presets.json`.

## 6. Display Locations

### 6a. Settings — Providers Section

**Collapsed state**: icon before accent bar in `prov-lhs`:

```
[icon 16px] [accent bar] [name / badge]
```

**Editing/Adding state**: icon before name input:

```
[icon 16px] [name input] [preset button]
```

**Preset dropdown**: icon before each option:

```
[icon 14px] OpenAI
           https://api.openai.com/v1
```

### 6b. Settings — Mode Model Selectors

Translation and Sparkle model dropdowns, icon before `opt-info`:

```
[icon 14px] gpt-4o-mini
           OpenAI
```

Icon resolved from `allFlat` entry's provider → preset list → `icon` field.

### 6c. Onboarding — Preset Selector

Same style as Settings preset dropdown:

```
[icon 14px] OpenAI
           https://api.openai.com/v1
```

All three locations use CSS `color` from parent for theme adaptation.

## 8. Existing Data Updates

All current entries in `provider-presets.json` must be updated with the `icon` field:

| name | icon |
|------|------|
| Custom | `""` (Cloud fallback) |
| OpenAI | `"openai"` |
| DeepSeek | `"deepseek"` |
| Kimi | `"kimi"` |
| Zhipu (BigModel) | `"zhipu"` |
| MINIMAX (CN) | `"minimax"` |
| Ollama (local) | `"ollama"` |

## 9. About Page — Acknowledgments

Add `@lobehub/icons` to the `deps` array in `src/views/About.vue`:

```ts
{ name: "Lobe Icons", version: "1.x", url: "https://www.npmjs.com/package/@lobehub/icons" },
```

## 7. Developer README

Located at `src/components/icons/providers/README.md`. Contains:

1. **Where** — the `providers/` directory
2. **How to add** — three steps:
   - Go to [lobehub.com/icons](https://lobehub.com/icons), find the provider, copy mono SVG source
   - Create `ProviderName.vue` from the template below, paste SVG paths, replace all `fill`/`stroke` with `currentColor`
   - Add one line to `index.ts` map: `"key": Component`
3. **How to bind** — add `"icon": "key"` to the corresponding entry in `provider-presets.json`
4. **That's it** — no other code changes needed; all display locations pick it up automatically
5. **Naming** — map key = lowercase (`openai`), file = PascalCase (`OpenAI.vue`)

**Template** (included in README for copy-paste):

```vue
<template>
  <svg :width="size" :height="size" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
    <!-- Adjust viewBox per original SVG -->
    <!-- Replace path below with icon content, all fill/stroke → currentColor -->
    <path d="..." fill="currentColor" />
  </svg>
</template>

<script setup lang="ts">
defineProps<{ size?: number }>();
</script>
```
