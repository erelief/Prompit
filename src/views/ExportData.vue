<script setup lang="ts">
import { ref, computed } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { save } from "@tauri-apps/plugin-dialog";
import { useSettingsWindow } from "../composables/useSettingsWindow";
import DataCategorySelector from "../components/DataCategorySelector.vue";
import { ALL_CATEGORIES, defaultSelectedCategories } from "../composables/useDataCategories";
import { Upload, Eye, EyeOff, ArrowLeft } from "@lucide/vue";

const { t } = useI18n();
const router = useRouter();
const { growAbove } = useSettingsWindow();

type Status = { kind: "idle" | "info" | "success" | "error"; msg: string };
const JSON_FILTER = [{ name: "JSON", extensions: ["json"] }];

// Selected categories default to all. The backend filters the bundle to this
// set; an empty selection is treated as "all known" by the backend, but we keep
// at least the default-checked UX here.
const selected = ref<string[]>([...defaultSelectedCategories("export")]);

const exportPassword = ref("");
const exportConfirmPassword = ref("");
const exportShowPw = ref(false);
const exportConfirmShowPw = ref(false);
const exportStatus = ref<Status>({ kind: "idle", msg: "" });
const exportBusy = ref(false);

const exportReady = computed(
  () => exportPassword.value.length >= 6
    && exportPassword.value === exportConfirmPassword.value
    && selected.value.length > 0,
);

function resetExport() {
  exportPassword.value = "";
  exportConfirmPassword.value = "";
  exportShowPw.value = false;
  exportConfirmShowPw.value = false;
}

async function handleExport() {
  if (!exportReady.value) return;
  const path = await save({
    defaultPath: `prompit-backup-${todayStamp()}.json`,
    filters: JSON_FILTER,
  });
  if (!path) {
    exportStatus.value = { kind: "info", msg: t("settings.exportData.export.cancelled") };
    return;
  }
  exportBusy.value = true;
  try {
    await invoke("export_data", {
      path,
      password: exportPassword.value,
      categories: selected.value,
    });
    exportStatus.value = { kind: "success", msg: t("settings.exportData.export.success", { path }) };
    resetExport();
  } catch (err) {
    exportStatus.value = {
      kind: "error",
      msg: t("settings.exportData.error", { message: String(err) }),
    };
  } finally {
    exportBusy.value = false;
  }
}

async function handleDrag(e: MouseEvent) {
  const target = e.target as HTMLElement;
  if (target.closest("button, input, textarea, a, select, .ud-footer")) return;
  await getCurrentWindow().startDragging();
}

function todayStamp(): string {
  const d = new Date();
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${mm}-${dd}`;
}
</script>

<template>
  <div class="ud-root" :class="{ 'grow-above': growAbove }" @mousedown="handleDrag">
    <!-- Header -->
    <div class="ud-header">
      <button class="back-btn" @click="router.push('/settings?tab=general')">
        <ArrowLeft :size="16" />
      </button>
      <span class="header-title">{{ t('settings.exportData.pageTitle') }}</span>
    </div>

    <!-- Body -->
    <div class="ud-body">
      <p class="ud-desc">{{ t('settings.exportData.export.description') }}</p>

      <!-- Category selector -->
      <div class="selector-label">{{ t('settings.exportData.selectCategories') }}</div>
      <DataCategorySelector v-model="selected" :available="ALL_CATEGORIES" />

      <!-- password -->
      <div class="pw-row">
        <input
          :type="exportShowPw ? 'text' : 'password'"
          class="pw-input"
          v-model="exportPassword"
          :placeholder="t('settings.exportData.export.passwordPlaceholder')"
          autocomplete="new-password"
        />
        <button class="pw-toggle" @click="exportShowPw = !exportShowPw" type="button">
          <Eye v-if="!exportShowPw" :size="13" />
          <EyeOff v-else :size="13" />
        </button>
      </div>

      <!-- confirm password -->
      <div
        class="pw-row"
        :class="{ mismatch: exportConfirmPassword.length > 0 && exportConfirmPassword !== exportPassword }"
      >
        <input
          :type="exportConfirmShowPw ? 'text' : 'password'"
          class="pw-input"
          v-model="exportConfirmPassword"
          :placeholder="t('settings.exportData.export.confirmPasswordPlaceholder')"
          autocomplete="new-password"
        />
        <button class="pw-toggle" @click="exportConfirmShowPw = !exportConfirmShowPw" type="button">
          <Eye v-if="!exportConfirmShowPw" :size="13" />
          <EyeOff v-else :size="13" />
        </button>
      </div>

      <button
        class="ud-btn primary-btn"
        :disabled="!exportReady || exportBusy"
        @click="handleExport"
      >
        <Upload :size="12" :stroke-width="1.9" />{{ t('settings.exportData.export.button') }}
      </button>
      <p v-if="!exportReady" class="ud-hint">{{ t('settings.exportData.export.hint') }}</p>
    </div>

    <!-- Footer status -->
    <div class="ud-footer">
      <span
        v-if="exportStatus.kind !== 'idle'"
        class="status-text"
        :class="{
          success: exportStatus.kind === 'success',
          error: exportStatus.kind === 'error',
          info: exportStatus.kind === 'info',
        }"
      >{{ exportStatus.msg }}</span>
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
.ud-root.grow-above .ud-header { order: 2; border-bottom: none; border-top: 1px solid var(--color-surface); }
.ud-root.grow-above .ud-footer { order: 1; border-top: none; border-bottom: 1px solid var(--color-surface); }
.ud-root.grow-above .ud-body { order: 0; }

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
  border: none;
  background: none;
  cursor: pointer;
}
.back-btn:hover {
  color: var(--color-text);
  background: var(--color-surface-hover);
}

.ud-body {
  flex: 1;
  overflow-y: auto;
  padding: 18px 24px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.ud-body::-webkit-scrollbar { width: 3px; }
.ud-body::-webkit-scrollbar-thumb { background: var(--color-scrollbar); border-radius: 3px; }

.ud-desc {
  font-size: 11px;
  font-weight: 500;
  line-height: 1.55;
  color: var(--color-text-muted);
}
.ud-hint {
  font-size: 10px;
  font-weight: 500;
  color: var(--color-text-muted);
  opacity: 0.8;
}
.selector-label {
  font-size: 10.5px;
  font-weight: 650;
  color: var(--color-text-secondary);
  letter-spacing: 0.01em;
  margin-top: 2px;
}

.pw-row {
  display: flex;
  align-items: center;
  gap: 6px;
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: 8px;
  padding: 7px 10px;
  transition: border-color 0.15s, box-shadow 0.15s;
}
.pw-row:focus-within {
  border-color: var(--color-accent-border);
  box-shadow: 0 0 0 2px var(--color-accent-bg);
}
.pw-row.mismatch {
  border-color: var(--color-danger);
}
.pw-input {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  color: var(--color-text);
  font-size: 12px;
  font-family: inherit;
  min-width: 0;
}
.pw-input::placeholder { color: var(--color-text-placeholder); }
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
  flex-shrink: 0;
}
.pw-toggle:hover:not(:disabled) {
  color: var(--color-text);
  background: var(--color-border);
}

.ud-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  padding: 6px 14px;
  border-radius: 8px;
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
  border: 1px solid transparent;
  background: none;
  transition: 0.15s;
  font-family: inherit;
}
.ud-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.primary-btn {
  color: var(--color-text);
  background: var(--color-surface-hover);
  border-color: var(--color-border);
}
.primary-btn:hover:not(:disabled) {
  background: var(--color-border);
}

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
.status-text.success { color: var(--color-success); }
.status-text.error { color: var(--color-danger); }
.status-text.info { color: var(--color-text-muted); }
</style>
