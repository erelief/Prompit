<script setup lang="ts">
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { ArrowLeft } from "@lucide/vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useSettingsWindow } from "../composables/useSettingsWindow";
// Vite resolves ?raw imports to the file's verbatim text content at build
// time, so the license text is bundled into the app and always in sync with
// the project's LICENSE file. The path is relative to the vite config root
// (the project root), which is where LICENSE lives.
import licenseText from "../../LICENSE?raw";

const { t } = useI18n();
const router = useRouter();
const { growAbove } = useSettingsWindow();

async function goBack() {
  router.push("/settings/about");
}

async function handleDrag(e: MouseEvent) {
  const target = e.target as HTMLElement;
  if (target.closest("button, a, .license-body")) return;
  await getCurrentWindow().startDragging();
}
</script>

<template>
  <div class="license-root" :class="{ 'grow-above': growAbove }" @mousedown="handleDrag">
    <!-- Header -->
    <div class="license-header">
      <button class="back-btn" @click="goBack">
        <ArrowLeft :size="16" />
      </button>
      <span class="header-title">{{ t('about.license') }}</span>
    </div>

    <!-- License body -->
    <main class="license-body">
      <pre class="license-text">{{ licenseText }}</pre>
    </main>
  </div>
</template>

<style scoped>
/* ══════════════════════════════════════
   License — full-text license page
   ══════════════════════════════════════ */
.license-root {
  height: calc(100dvh / var(--font-scale, 1));
  display: flex;
  flex-direction: column;
  background: var(--color-bg);
  color: var(--color-text);
  overflow: hidden;
  border-radius: var(--radius-lg);
}
.license-root.grow-above .license-header {
  order: 99;
  border-bottom: none;
  border-top: 1px solid var(--color-surface);
  margin-top: auto;
}

/* ── Header ── */
.license-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px 24px 12px;
  border-bottom: 1px solid var(--color-surface);
  flex-shrink: 0;
}

/* ── Body ── */
.license-body {
  flex: 1;
  overflow-y: auto;
  padding: 16px 24px;
  scrollbar-width: thin;
  scrollbar-color: var(--color-scrollbar) transparent;
}
.license-body::-webkit-scrollbar {
  width: 3px;
}
.license-body::-webkit-scrollbar-thumb {
  background: var(--color-scrollbar);
  border-radius: 3px;
}
.license-text {
  margin: 0;
  font-family: "Geist", -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif;
  font-size: 12px;
  line-height: 1.6;
  color: var(--color-text-secondary);
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
