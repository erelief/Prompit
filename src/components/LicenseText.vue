<script setup lang="ts">
import { useRouter } from "vue-router";
import { ArrowLeft } from "@lucide/vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useSettingsWindow } from "../composables/useSettingsWindow";

// Single component shared by the License + ThirdPartyLicenses routes.
// Previously two near-identical files (same template, same <style scoped>
// block — the only difference was the ?raw import and the i18n key).

defineProps<{
  /** Page title shown in the header. */
  title: string;
  /** Verbatim license text to render in the <pre>. */
  text: string;
}>();

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
      <span class="header-title">{{ title }}</span>
    </div>

    <!-- License body -->
    <main class="license-body">
      <pre class="license-text">{{ text }}</pre>
    </main>
  </div>
</template>

<style scoped>
/* ══════════════════════════════════════
   Full-text license page (shared by License + ThirdPartyLicenses routes)
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
  gap: var(--space-3);
  padding: var(--space-4) var(--space-6) var(--space-3);
  border-bottom: 1px solid var(--color-surface);
  flex-shrink: 0;
}

/* ── Body ── */
.license-body {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-4) var(--space-6);
  scrollbar-width: thin;
  scrollbar-color: var(--color-scrollbar) transparent;
}
.license-body::-webkit-scrollbar {
  width: 3px;
}
.license-body::-webkit-scrollbar-thumb {
  background: var(--color-scrollbar);
  border-radius: var(--radius-xs);
}
.license-text {
  margin: 0;
  font-family: var(--font-sans);
  font-size: var(--text-base);
  line-height: 1.6;
  color: var(--color-text-secondary);
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
