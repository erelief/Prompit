# Provider Icons Developer Guide

All provider brand icons live in this directory. They are monochrome SVG components using `currentColor`, bundled locally for full offline use.

Icons are sourced from [@lobehub/icons](https://www.npmjs.com/package/@lobehub/icons) (mono variant).

## Adding a New Provider Icon

### Step 1: Get the SVG

1. Go to [lobehub.com/icons](https://lobehub.com/icons)
2. Find your provider
3. Copy the **mono** SVG source code
4. Extract the `<path d="...">` content

### Step 2: Create the Vue Component

Create a new `.vue` file in this directory using this template:

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

**Rules:**
- All `fill` / `stroke` attributes must be `currentColor`
- Keep the original `viewBox`
- Accept optional `size` prop (default handled by parent)

### Step 3: Register in the Map

Edit `index.ts`:

```ts
// 1. Import at top
import NewProvider from './NewProvider.vue'

// 2. Add to PROVIDER_ICONS map
export const PROVIDER_ICONS: Record<string, Component> = {
  // ...existing entries
  newprovider: NewProvider,
}
```

### Step 4: Bind to Preset

Edit `provider-presets.json` (project root):

```json
{
  "name": "NewProvider",
  "icon": "newprovider",
  ...
}
```

That's it. All display locations (Settings, Onboarding, FloatingInput) pick up the icon automatically.

## Naming Convention

| What | Convention | Example |
|------|-----------|---------|
| Map key | lowercase | `openai`, `deepseek` |
| File name | PascalCase | `OpenAI.vue`, `DeepSeek.vue` |

## Existing Icons

| Key | File | Provider |
|-----|------|----------|
| openai | OpenAI.vue | OpenAI |
| deepseek | DeepSeek.vue | DeepSeek |
| kimi | Kimi.vue | Kimi (Moonshot AI) |
| zhipu | Zhipu.vue | Zhipu (BigModel) |
| minimax | Minimax.vue | MINIMAX (CN) |
| ollama | Ollama.vue | Ollama (local) |

## Fallback

When no icon matches (Custom provider, missing icon field), the component renders a Lucide `Cloud` icon automatically.
