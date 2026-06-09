<script setup lang="ts">
import { onMounted } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { ArrowLeft, ExternalLink } from "@lucide/vue";
import { useSettingsWindow } from "../composables/useSettingsWindow";

declare const __APP_VERSION__: string;
const appVersion = __APP_VERSION__;

const { t } = useI18n();
const router = useRouter();
const { growAbove } = useSettingsWindow();

const deps = [
  { name: "Tauri", version: "2.11.0", url: "https://tauri.app" },
  { name: "Vue", version: "3.5.35", url: "https://vuejs.org" },
  { name: "Vue Router", version: "5.0.7", url: "https://router.vuejs.org" },
  { name: "Vue I18n", version: "11.4.4", url: "https://vue-i18n.intlify.dev" },
  { name: "VueUse", version: "14.3.0", url: "https://vueuse.org" },
  { name: "Lucide", version: "1.17.0", url: "https://lucide.dev" },
  { name: "Tailwind CSS", version: "4.3.0", url: "https://tailwindcss.com" },
  { name: "VueDraggable", version: "4.1.0", url: "https://sortablejs.github.io/vue.draggable.next/" },
];

const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

onMounted(() => {
  const container = document.querySelector(".about-root");
  if (container && isTauri) {
    import("@tauri-apps/plugin-shell").then(({ open }) => {
      container.querySelectorAll<HTMLAnchorElement>('a[href^="http"]').forEach((a) => {
        a.addEventListener("click", (e) => { e.preventDefault(); open(a.href); });
      });
    });
  }
});
</script>

<template>
  <div class="about-root" :class="{ 'grow-above': growAbove }">
    <!-- Header -->
    <div class="about-header">
      <button class="back-btn" @click="router.push('/settings?tab=general')">
        <ArrowLeft :size="16" />
      </button>
      <span class="header-title">{{ t('about.title') }}</span>
    </div>

    <!-- Title row -->
    <div class="about-title-row">
      <img class="about-icon" src="/prompit_logo.svg" alt="" />
      <span class="about-name">Prompit</span>
      <span class="about-version">v{{ appVersion }}</span>
    </div>

    <!-- Info -->
    <div class="about-section">
      <span class="about-label">{{ t('about.repository') }}</span>
      <a class="about-link" href="https://github.com/erelief/Prompit">
        <span>erelief/Prompit</span>
        <ExternalLink :size="10" :stroke-width="2" />
      </a>
    </div>
    <div class="about-section">
      <span class="about-label">{{ t('about.license') }}</span>
      <span class="about-value">MIT License</span>
    </div>

    <!-- Acknowledgments -->
    <div class="about-divider" />
    <div class="about-deps-title">{{ t('about.acknowledgments') }}</div>
    <div class="about-dep" v-for="dep in deps" :key="dep.name">
      <span class="about-dep-name">{{ dep.name }}</span>
      <a class="about-link" :href="dep.url">
        <span>v{{ dep.version }}</span>
        <ExternalLink :size="10" :stroke-width="2" />
      </a>
    </div>
  </div>
</template>

<style scoped>
/* ══════════════════════════════════════
   About — info-only page
   ══════════════════════════════════════ */
.about-root {
  height: 100dvh;
  display: flex;
  flex-direction: column;
  background: var(--color-bg);
  color: var(--color-text);
  overflow: hidden;
  border-radius: 11px;
}
.about-root.grow-above .about-header { order: 99; border-bottom: none; border-top: 1px solid var(--color-surface); margin-top: auto; }

/* ── Header ── */
.about-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px 24px 12px;
  border-bottom: 1px solid var(--color-surface);
  flex-shrink: 0;
}
.header-title {
  flex: 1;
  font-size: 15px;
  font-weight: 700;
  letter-spacing: -0.02em;
  color: var(--color-text);
  line-height: 1.2;
}
.back-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 9px;
  color: var(--color-text-muted);
  border: none;
  background: none;
  cursor: pointer;
  transition: 0.15s;
}
.back-btn:hover {
  color: var(--color-text);
  background: var(--color-surface-hover);
}

/* ── Title row ── */
.about-title-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px 24px 8px;
  flex-shrink: 0;
}
.about-icon {
  height: 1.2em;
  width: auto;
  flex-shrink: 0;
}
.about-name {
  font-size: 14px;
  font-weight: 700;
  letter-spacing: -0.01em;
  color: var(--color-text);
}
.about-version {
  margin-left: auto;
  font-size: 12px;
  font-weight: 500;
  color: var(--color-text-muted);
  font-variant-numeric: tabular-nums;
}

/* ── Info sections ── */
.about-section {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 24px;
}
.about-label {
  font-size: 12px;
  color: var(--color-text-muted);
}
.about-value {
  font-size: 12px;
  color: var(--color-text-secondary);
  font-weight: 500;
}
.about-link {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  color: var(--color-accent-text);
  text-decoration: none;
  font-weight: 500;
  transition: 0.15s;
}
.about-link:hover {
  color: var(--color-accent);
}

/* ── Divider ── */
.about-divider {
  height: 1px;
  background: var(--color-surface);
  margin: 8px 24px;
}

/* ── Dependencies ── */
.about-deps-title {
  font-size: 11px;
  font-weight: 650;
  color: var(--color-text-muted);
  letter-spacing: 0.02em;
  padding: 2px 24px 6px;
}
.about-dep {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 3px 24px;
}
.about-dep-name {
  font-size: 12px;
  color: var(--color-text-muted);
}
</style>
