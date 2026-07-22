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
import { appConfig } from "../stores/config";
import { Upload, CloudUpload, Eye, EyeOff, ArrowLeft } from "@lucide/vue";

const { t } = useI18n();
const router = useRouter();
const { growAbove } = useSettingsWindow();

type Status = { kind: "idle" | "info" | "success" | "error"; msg: string };
const JSON_FILTER = [{ name: "JSON", extensions: ["json"] }];

// Selected categories default to all. The backend filters the bundle to this
// set; an empty selection is treated as "all known" by the backend, but we keep
// at least the default-checked UX here.
const selected = ref<string[]>([...defaultSelectedCategories("export")]);

// Backup file name (no extension; `.json` is appended at the point of use).
// Mainly affects the WebDAV upload, which has no OS save dialog — for local
// saves it only pre-fills the file-dialog default and the user can rename it.
const fileName = ref(`prompit_config_${shortDateStamp()}`);
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

const passwordMismatch = computed(
  () => exportConfirmPassword.value.length > 0
    && exportConfirmPassword.value !== exportPassword.value,
);

// The safekeeping warning only matters once a password has actually been set —
// surface it after the user has filled in both fields.
const showPasswordWarning = computed(
  () => exportConfirmPassword.value.length > 0,
);

// WebDAV is just another destination for the same bundle — available only
// once a server is configured (Settings → General → Data Management → WebDAV).
const webdavConfigured = computed(() => appConfig.webdav.url.trim().length > 0);

function resetExport() {
  exportPassword.value = "";
  exportConfirmPassword.value = "";
  exportShowPw.value = false;
  exportConfirmShowPw.value = false;
}

async function handleExport() {
  if (!exportReady.value) return;
  const path = await save({
    defaultPath: jsonFileName(),
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

async function handleExportWebdav() {
  if (!exportReady.value || !webdavConfigured.value) return;
  exportBusy.value = true;
  try {
    const r = await invoke<{ bytes: number }>("webdav_export", {
      password: exportPassword.value,
      categories: selected.value,
      fileName: jsonFileName(),
    });
    const kb = Math.max(1, Math.round(r.bytes / 1024));
    exportStatus.value = {
      kind: "success",
      msg: t("settings.exportData.export.webdavSuccess", { size: kb }),
    };
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
  if (target.closest("button, input, textarea, a, select")) return;
  await getCurrentWindow().startDragging();
}

/** Compact date stamp YYMMDD (2-digit year + zero-padded month/day). */
function shortDateStamp(): string {
  const d = new Date();
  const yy = String(d.getFullYear()).slice(-2);
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${yy}${mm}${dd}`;
}

/** The `.json` file name to use, derived from the user's input. The backend
 *  appends `.json` if missing, so this only needs to supply the stem and fall
 *  back to the dated default when the field is empty. */
function jsonFileName(): string {
  const base = fileName.value.trim();
  return `${base || `prompit_config_${shortDateStamp()}`}.json`;
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
      <div class="field-label">{{ t('settings.exportData.export.passwordLabel') }}</div>
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
        :class="{ mismatch: passwordMismatch }"
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
      <p v-if="showPasswordWarning" class="ud-warn">{{ t('settings.exportData.export.passwordWarning') }}</p>
      <p v-if="passwordMismatch" class="ud-hint status-text error">{{ t('settings.exportData.export.passwordMismatch') }}</p>

      <!-- backup file name (mainly affects WebDAV upload) -->
      <div class="field-label">{{ t('settings.exportData.fileNameLabel') }}</div>
      <div class="pw-row">
        <input
          class="pw-input"
          v-model="fileName"
          :placeholder="t('settings.exportData.fileNamePlaceholder')"
          autocomplete="off"
          spellcheck="false"
        />
      </div>
      <p class="ud-hint">{{ t('settings.exportData.fileNameHint') }}</p>

      <div class="btn-row">
        <button
          class="ud-btn primary-btn"
          :disabled="!exportReady || exportBusy"
          @click="handleExport"
        >
          <Upload :size="12" :stroke-width="1.9" />{{ exportBusy ? t('settings.exportData.export.exporting') : t('settings.exportData.export.toFile') }}
        </button>
        <button
          class="ud-btn secondary-btn"
          :disabled="!exportReady || exportBusy || !webdavConfigured"
          :title="webdavConfigured ? '' : t('settings.webdav.notConfigured')"
          @click="handleExportWebdav"
        >
          <CloudUpload :size="12" :stroke-width="1.9" />{{ t('settings.exportData.export.toWebdav') }}
        </button>
      </div>
      <p v-if="!exportReady" class="ud-hint">{{ t('settings.exportData.export.hint') }}</p>
      <p v-else-if="!webdavConfigured" class="ud-hint">{{ t('settings.webdav.notConfigured') }}</p>

      <p
        v-if="exportStatus.kind !== 'idle'"
        class="status-text"
        :class="{
          success: exportStatus.kind === 'success',
          error: exportStatus.kind === 'error',
          info: exportStatus.kind === 'info',
        }"
      >{{ exportStatus.msg }}</p>
    </div>
  </div>
</template>

<style scoped>
.secondary-btn {
  color: var(--color-text-secondary);
  border-color: var(--color-border);
  background: transparent;
}
.secondary-btn:hover:not(:disabled) {
  background: var(--color-surface-hover);
  color: var(--color-text);
}
</style>
