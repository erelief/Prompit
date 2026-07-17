<script setup lang="ts">
import { onMounted } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { ArrowLeft, ExternalLink, ChevronRight } from "@lucide/vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useSettingsWindow } from "../composables/useSettingsWindow";
// Versions are resolved from package-lock.json / Cargo.lock at build time by
// scripts/generate-third-party-licenses.mjs. To add/remove an entry, edit the
// ABOUT_DEPS list in that script and re-run `npm run build` (or
// `npm run gen:third-party`). Fonts and manually-vendored assets aren't in the
// lockfile as production deps, so they are appended below.
import generatedDeps from "../generated/about-deps.json";

declare const __APP_VERSION__: string;
const appVersion = __APP_VERSION__;

const { t } = useI18n();
const router = useRouter();
const { growAbove } = useSettingsWindow();

const deps = [
  ...generatedDeps,
  // ── Fonts ──
  // Geist ships the woff2 bundled in src/assets/fonts/. It's a devDep in the
  // lockfile (build-time only) so the generator skips it — acknowledge here.
  { name: "Geist", version: "1.7.2", url: "https://vercel.com/font" },
  // Madimi One renders the "P" in the logo SVG. OFL-1.1 (see OFL FAQ). Not an
  // npm/cargo dep, so it isn't in any lockfile.
  { name: "Madimi One", version: "", url: "https://fonts.google.com/specimen/Madimi+One" },
  // ── Manually-vendored assets ──
  { name: "Lobe Icons", version: "1.91.0", url: "https://www.npmjs.com/package/@lobehub/icons" },
];

const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function handleDrag(e: MouseEvent) {
  const target = e.target as HTMLElement;
  if (target.closest("button, a, input")) return;
  await getCurrentWindow().startDragging();
}

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
  <div class="about-root" :class="{ 'grow-above': growAbove }" @mousedown="handleDrag">
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
      <button class="about-link" @click="router.push('/settings/license')">
        <span>Apache License 2.0</span>
        <ChevronRight :size="12" :stroke-width="2" />
      </button>
    </div>
    <div class="about-section">
      <span class="about-label">{{ t('about.thirdParty') }}</span>
      <button class="about-link" @click="router.push('/settings/third-party')">
        <span>{{ t('common.view') }}</span>
        <ChevronRight :size="12" :stroke-width="2" />
      </button>
    </div>

    <!-- Acknowledgments -->
    <div class="about-divider" />
    <div class="about-deps-title">{{ t('about.acknowledgments') }}</div>
    <div class="about-deps-scroll">
      <div class="about-dep" v-for="dep in deps" :key="dep.name">
        <span class="about-dep-name">{{ dep.name }}</span>
        <a class="about-link" :href="dep.url">
          <span v-if="dep.version">v{{ dep.version }}</span>
          <span v-else>{{ dep.name }}</span>
          <ExternalLink :size="10" :stroke-width="2" />
        </a>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* ══════════════════════════════════════
   About — info-only page
   ══════════════════════════════════════ */
.about-root {
  height: calc(100dvh / var(--font-scale, 1));
  display: flex;
  flex-direction: column;
  background: var(--color-bg);
  color: var(--color-text);
  overflow: hidden;
  border-radius: var(--radius-lg);
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
  flex-shrink: 0;
}
.about-label {
  font-size: 12px;
  color: var(--color-text-muted);
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
/* When used as a <button> (the license row), reset button defaults so it
   renders identically to the <a> variant above. */
button.about-link {
  border: none;
  background: none;
  padding: 0;
  cursor: pointer;
  font-family: inherit;
}
.about-link:hover {
  color: var(--color-accent);
}

/* ── Divider ── */
.about-divider {
  height: 1px;
  background: var(--color-surface);
  margin: 8px 24px;
  flex-shrink: 0;
}

/* ── Dependencies ── */
.about-deps-title {
  font-size: 11px;
  font-weight: 650;
  color: var(--color-text-muted);
  letter-spacing: 0.02em;
  padding: 2px 24px 6px;
  flex-shrink: 0;
}
/* Scrollable acknowledgments list: takes remaining height, scrolls internally
   so the header/title/info rows stay fixed and never get pushed off-screen. */
.about-deps-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding-bottom: 8px;
  scrollbar-width: thin;
  scrollbar-color: var(--color-scrollbar) transparent;
}
.about-deps-scroll::-webkit-scrollbar {
  width: 3px;
}
.about-deps-scroll::-webkit-scrollbar-thumb {
  background: var(--color-scrollbar);
  border-radius: 3px;
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
