<script setup lang="ts">
import { ref } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useSettingsWindow } from "../composables/useSettingsWindow";
import { ArrowLeft, Download, Upload, Eye, EyeOff } from "@lucide/vue";

const { t } = useI18n();
const router = useRouter();
const { growAbove } = useSettingsWindow();

// ── Export state ──
const exportPassword = ref("");
const exportShowPw = ref(false);
const exportStatus = ref<{ kind: "idle" | "info" | "success" | "error"; msg: string }>({ kind: "idle", msg: "" });
const exportBusy = ref(false);

// ── Import state ──
const importPassword = ref("");
const importShowPw = ref(false);
const importStatus = ref<{ kind: "idle" | "info" | "success" | "error"; msg: string }>({ kind: "idle", msg: "" });
const importBusy = ref(false);

async function handleDrag(e: MouseEvent) {
  const target = e.target as HTMLElement;
  if (target.closest("button, input, textarea, a, select, .ud-section, .ud-footer")) return;
  await getCurrentWindow().startDragging();
}

function todayStamp(): string {
  const d = new Date();
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${mm}-${dd}`;
}

async function handleExport() {
  if (exportPassword.value.length < 6) {
    exportStatus.value = { kind: "error", msg: t("settings.userData.passwordTooShort") };
    return;
  }
  const path = await save({
    defaultPath: `prompit-backup-${todayStamp()}.json`,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (!path) {
    exportStatus.value = { kind: "info", msg: t("settings.userData.export.cancelled") };
    return;
  }
  exportBusy.value = true;
  try {
    await invoke("export_data", { path, password: exportPassword.value });
    exportStatus.value = { kind: "success", msg: t("settings.userData.export.success", { path }) };
    exportPassword.value = "";
  } catch (err) {
    exportStatus.value = { kind: "error", msg: t("settings.userData.error", { message: String(err) }) };
  } finally {
    exportBusy.value = false;
  }
}

async function handleImport() {
  if (importPassword.value.length === 0) {
    importStatus.value = { kind: "error", msg: t("settings.userData.passwordRequired") };
    return;
  }
  const selected = await open({
    multiple: false,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  const path = typeof selected === "string" ? selected : null;
  if (!path) {
    importStatus.value = { kind: "info", msg: t("settings.userData.import.cancelled") };
    return;
  }
  importBusy.value = true;
  try {
    await invoke("import_data", { path, password: importPassword.value });
    importStatus.value = { kind: "success", msg: t("settings.userData.import.success") };
    importPassword.value = "";
  } catch (err) {
    importStatus.value = { kind: "error", msg: t("settings.userData.error", { message: String(err) }) };
  } finally {
    importBusy.value = false;
  }
}
</script>

<template>
  <div class="ud-root" :class="{ 'grow-above': growAbove }" @mousedown="handleDrag">
    <!-- Header -->
    <div class="ud-header">
      <button class="back-btn" @click="router.push('/settings?tab=general')">
        <ArrowLeft :size="16" />
      </button>
      <span class="header-title">{{ t('settings.userData.pageTitle') }}</span>
    </div>

    <!-- Body -->
    <div class="ud-body settings-scrollbar">
      <!-- Export section -->
      <section class="ud-section">
        <div class="section-head">
          <span class="section-title"><Download :size="13" />{{ t('settings.userData.export.title') }}</span>
        </div>
        <p class="ud-desc">{{ t('settings.userData.export.description') }}</p>
        <div class="pw-row">
          <input
            :type="exportShowPw ? 'text' : 'password'"
            class="pw-input"
            v-model="exportPassword"
            :placeholder="t('settings.userData.export.passwordPlaceholder')"
            autocomplete="new-password"
          />
          <button class="pw-toggle" @click="exportShowPw = !exportShowPw" type="button">
            <Eye v-if="!exportShowPw" :size="13" />
            <EyeOff v-else :size="13" />
          </button>
        </div>
        <button class="ud-action" :disabled="exportBusy" @click="handleExport">
          <Download :size="12" :stroke-width="1.9" />{{ t('settings.userData.export.button') }}
        </button>
      </section>

      <!-- Import section -->
      <section class="ud-section">
        <div class="section-head">
          <span class="section-title"><Upload :size="13" />{{ t('settings.userData.import.title') }}</span>
        </div>
        <p class="ud-desc">{{ t('settings.userData.import.description') }}</p>
        <p class="ud-warn">{{ t('settings.userData.import.warning') }}</p>
        <div class="pw-row">
          <input
            :type="importShowPw ? 'text' : 'password'"
            class="pw-input"
            v-model="importPassword"
            :placeholder="t('settings.userData.import.passwordPlaceholder')"
            autocomplete="off"
          />
          <button class="pw-toggle" @click="importShowPw = !importShowPw" type="button">
            <Eye v-if="!importShowPw" :size="13" />
            <EyeOff v-else :size="13" />
          </button>
        </div>
        <button class="ud-action danger" :disabled="importBusy" @click="handleImport">
          <Upload :size="12" :stroke-width="1.9" />{{ t('settings.userData.import.button') }}
        </button>
      </section>
    </div>

    <!-- Footer status -->
    <div class="ud-footer">
      <span
        v-if="exportStatus.kind !== 'idle'"
        class="status-text"
        :class="{ success: exportStatus.kind === 'success', error: exportStatus.kind === 'error' }"
      >{{ exportStatus.msg }}</span>
      <span
        v-if="importStatus.kind !== 'idle'"
        class="status-text"
        :class="{ success: importStatus.kind === 'success', error: importStatus.kind === 'error' }"
      >{{ importStatus.msg }}</span>
    </div>
  </div>
</template>

<style scoped>
.ud-root {
  height: calc(100dvh / var(--font-scale, 1));
  display: flex;
  flex-direction: column;
  background: var(--color-bg);
  color: var(--color-text);
  overflow: hidden;
  border-radius: 11px;
}

/* ── Header (mirrors ResetSoftware) ── */
.ud-header {
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
  transition: 0.15s;
}
.back-btn:hover {
  color: var(--color-text);
  background: var(--color-surface-hover);
}

/* ── Body ── */
.ud-body {
  flex: 1;
  overflow-y: auto;
  padding: 20px 24px;
  display: flex;
  flex-direction: column;
  gap: 22px;
}
.ud-section {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.section-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 2px;
}
.section-title {
  display: flex;
  align-items: center;
  gap: 7px;
  font-size: 11.5px;
  font-weight: 650;
  letter-spacing: 0.01em;
  color: var(--color-text-secondary);
}
.ud-desc {
  font-size: 11.5px;
  font-weight: 500;
  line-height: 1.55;
  color: var(--color-text-muted);
}
.ud-warn {
  font-size: 11px;
  font-weight: 600;
  color: var(--color-danger);
  letter-spacing: 0.01em;
}

/* ── Password input ── */
.pw-row {
  display: flex;
  align-items: center;
  gap: 6px;
  background: var(--color-surface);
  border: 1px solid var(--color-scrollbar);
  border-radius: 9px;
  padding: 8px 10px;
}
.pw-input {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  color: var(--color-text);
  font-size: 12px;
  font-family: inherit;
}
.pw-input::placeholder { color: var(--color-text-muted); }
.pw-toggle {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: 6px;
  color: var(--color-text-muted);
  border: none;
  background: none;
  cursor: pointer;
  transition: 0.12s;
}
.pw-toggle:hover { color: var(--color-text); background: var(--color-border); }

/* ── Action button ── */
.ud-action {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  align-self: flex-start;
  padding: 5px 12px;
  border-radius: 8px;
  font-size: 11px;
  font-weight: 600;
  color: var(--color-text);
  background: var(--color-surface-hover);
  border: 1px solid var(--color-border);
  cursor: pointer;
  transition: 0.15s;
}
.ud-action:hover:not(:disabled) {
  background: var(--color-border);
}
.ud-action.danger {
  color: var(--color-danger);
  border-color: var(--color-danger-bg);
}
.ud-action.danger:hover:not(:disabled) {
  background: var(--color-danger-bg);
}
.ud-action:disabled { opacity: 0.5; cursor: not-allowed; }

/* ── Footer ── */
.ud-footer {
  flex-shrink: 0;
  padding: 10px 24px 14px;
  border-top: 1px solid var(--color-surface);
  min-height: 28px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.status-text {
  font-size: 10.5px;
  font-weight: 500;
  color: var(--color-text-muted);
}
.status-text.success { color: #16a34a; }
.status-text.error { color: var(--color-danger); }
</style>
