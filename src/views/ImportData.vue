<script setup lang="ts">
import { ref, computed } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useSettingsWindow } from "../composables/useSettingsWindow";
import { useDataImport } from "../composables/useDataImport";
import DataCategorySelector from "../components/DataCategorySelector.vue";
import { ALL_CATEGORIES, knownCategoriesIn } from "../composables/useDataCategories";
import { appConfig } from "../stores/config";
import {
  Download, ArrowLeft, Eye, EyeOff, Check, X, ShieldAlert,
  FileText, FolderOpen, Search, CloudUpload,
} from "@lucide/vue";

const { t } = useI18n();
const router = useRouter();
const { growAbove } = useSettingsWindow();

const {
  importPath, importSourceKind, importPassword, importShowPw, importConfirming,
  importCountdown, importStatus, importBusy,
  importPreview, importSelected, importAnalyzed, importAnalyzing,
  importFileName, importCanAnalyze, importCanConfirm,
  selectImportFile, selectWebdavFile, analyzeImport, requestImport, confirmImport, stopCountdown,
} = useDataImport({
  messages: {
    success: t("settings.importData.import.success"),
    error: (message: string) => t("settings.importData.error", { message }),
  },
  // No onSuccess override: the composable default hot-reloads config in-place.
});

// The category selector is bound to a string[] v-model; bridge it to the Set.
const selectedArray = computed<string[]>({
  get: () => [...importSelected.value],
  set: (v) => { importSelected.value = new Set(v); },
});

// Only categories present in the analyzed bundle are selectable.
const available = computed(() =>
  importAnalyzed.value ? knownCategoriesIn(importPreview.value) : ALL_CATEGORIES,
);

const analyzeBtnLabel = computed(() =>
  importAnalyzing.value
    ? t("settings.importData.import.analyzing")
    : importAnalyzed.value
      ? t("settings.importData.import.reanalyze")
      : t("settings.importData.import.analyze"),
);

// ── WebDAV source picking ─────────────────────────────────────────────────
// Available only once a server is configured; the Rust side reads the saved
// connection, so nothing sensitive passes through here.
const webdavConfigured = computed(() => appConfig.webdav.url.trim().length > 0);
const webdavPicking = ref(false);
const webdavFiles = ref<string[]>([]);
const webdavLoading = ref(false);
const webdavChoice = ref("");

async function openWebdavPicker() {
  webdavPicking.value = true;
  webdavLoading.value = true;
  webdavChoice.value = "";
  try {
    const files = await invoke<string[]>("webdav_list_files");
    webdavFiles.value = files;
    if (files.length === 0) {
      webdavPicking.value = false;
      importStatus.value = { kind: "info", msg: t("settings.importData.import.webdavEmpty") };
    }
  } catch (err) {
    webdavPicking.value = false;
    importStatus.value = {
      kind: "error",
      msg: t("settings.importData.error", { message: String(err) }),
    };
  } finally {
    webdavLoading.value = false;
  }
}

function pickWebdav() {
  if (!webdavChoice.value) return;
  selectWebdavFile(webdavChoice.value);
  webdavPicking.value = false;
  webdavChoice.value = "";
}

function cancelWebdavPicker() {
  webdavPicking.value = false;
  webdavChoice.value = "";
}

async function handleDrag(e: MouseEvent) {
  const target = e.target as HTMLElement;
  if (target.closest("button, input, textarea, a, select")) return;
  await getCurrentWindow().startDragging();
}
</script>

<template>
  <div class="ud-root" :class="{ 'grow-above': growAbove }" @mousedown="handleDrag">
    <!-- Header -->
    <div class="ud-header">
      <button class="back-btn" @click="router.push('/settings?tab=general')">
        <ArrowLeft :size="16" />
      </button>
      <span class="header-title">{{ t('settings.importData.pageTitle') }}</span>
    </div>

    <!-- Body -->
    <div class="ud-body">
      <p class="ud-desc">{{ t('settings.importData.import.description') }}</p>
      <p class="ud-warn">{{ t('settings.importData.import.warning') }}</p>

      <!-- Step 1: pick a source (local file or WebDAV server) -->
      <template v-if="!importPath">
        <div v-if="webdavPicking" class="webdav-pick-row">
          <select class="webdav-select" v-model="webdavChoice" :disabled="webdavLoading">
            <option value="" disabled>
              {{ webdavLoading
                ? t('settings.importData.import.webdavLoading')
                : t('settings.importData.import.webdavPickPlaceholder') }}
            </option>
            <option v-for="f in webdavFiles" :key="f" :value="f">{{ f }}</option>
          </select>
          <button
            class="mini-btn"
            :title="t('common.confirm')"
            :disabled="!webdavChoice"
            @click="pickWebdav"
          >
            <Check :size="12" :stroke-width="2.5" />
          </button>
          <button class="mini-btn" :title="t('common.cancel')" @click="cancelWebdavPicker">
            <X :size="12" :stroke-width="2.5" />
          </button>
        </div>
        <template v-else>
          <button class="file-pick-btn" @click="selectImportFile">
            <FolderOpen :size="14" :stroke-width="1.8" />{{ t('settings.importData.import.selectFile') }}
          </button>
          <button
            class="file-pick-btn"
            :disabled="!webdavConfigured"
            :title="webdavConfigured ? '' : t('settings.webdav.notConfigured')"
            @click="openWebdavPicker"
          >
            <CloudUpload :size="14" :stroke-width="1.8" />{{ t('settings.importData.import.fromWebdav') }}
          </button>
        </template>
      </template>

      <!-- Step 2/3: source selected -->
      <template v-else>
        <!-- selected source row -->
        <div class="file-row">
          <CloudUpload v-if="importSourceKind === 'webdav'" :size="14" class="file-icon" />
          <FileText v-else :size="14" class="file-icon" />
          <span class="file-name" :title="importPath || ''">{{ importFileName }}</span>
          <button
            class="pw-toggle change-file-btn"
            :disabled="importConfirming || importBusy || importAnalyzing"
            @click="importSourceKind === 'webdav' ? openWebdavPicker() : selectImportFile()"
            type="button"
          >
            <FolderOpen :size="13" />
          </button>
        </div>

        <!-- password + analyze (hidden while confirming) -->
        <template v-if="!importConfirming">
          <div class="pw-row">
            <input
              :type="importShowPw ? 'text' : 'password'"
              class="pw-input"
              v-model="importPassword"
              :placeholder="t('settings.importData.import.passwordPlaceholder')"
              autocomplete="off"
              @keyup.enter="analyzeImport"
            />
            <button class="pw-toggle" @click="importShowPw = !importShowPw" type="button">
              <Eye v-if="!importShowPw" :size="13" />
              <EyeOff v-else :size="13" />
            </button>
          </div>

          <button
            class="ud-btn analyze-btn"
            :disabled="!importCanAnalyze"
            @click="analyzeImport"
          >
            <Search :size="12" :stroke-width="1.9" />{{ analyzeBtnLabel }}
          </button>

          <!-- Category selector appears once analysis succeeds -->
          <template v-if="importAnalyzed">
            <div class="selector-label">{{ t('settings.importData.selectCategories') }}</div>
            <DataCategorySelector
              v-model="selectedArray"
              :available="available"
              :counts="importPreview"
            />
          </template>

          <!-- import button -->
          <button
            class="ud-btn primary-btn danger"
            :disabled="!importCanConfirm"
            @click="requestImport"
          >
            <Download :size="12" :stroke-width="1.9" />{{ t('settings.importData.import.button') }}
          </button>
        </template>

        <!-- Step 3: countdown confirm -->
        <div v-else class="confirm-warn-row">
          <div class="confirm-warn-text">
            <ShieldAlert :size="14" :stroke-width="1.6" />
            <span>{{ t('settings.importData.import.confirmWarning') }}</span>
          </div>
          <div class="confirm-actions">
            <button
              class="mini-btn"
              :title="t('common.cancel')"
              :disabled="importBusy"
              @click="stopCountdown"
            >
              <X :size="12" :stroke-width="2.5" />
            </button>
            <div class="confirm-with-countdown">
              <button
                class="mini-btn danger-active"
                :class="{ 'confirm-counting': importCountdown > 0 }"
                :title="importCountdown > 0
                  ? t('settings.reset.confirmCountdown', { n: importCountdown })
                  : t('common.confirm')"
                :disabled="importCountdown > 0 || importBusy"
                @click="confirmImport"
              >
                <Check :size="12" :stroke-width="2.5" />
              </button>
              <span v-if="importCountdown > 0" class="countdown-label">{{ importCountdown }}s</span>
            </div>
          </div>
        </div>
      </template>

      <p
        v-if="importStatus.kind !== 'idle'"
        class="status-text"
        :class="{
          success: importStatus.kind === 'success',
          error: importStatus.kind === 'error',
          info: importStatus.kind === 'info',
        }"
      >{{ importStatus.msg }}</p>
    </div>
  </div>
</template>

<style scoped>
.ud-warn {
  font-size: var(--text-sm);
  font-weight: var(--weight-semibold);
  color: var(--color-danger);
  letter-spacing: 0.01em;
}

.change-file-btn { width: 22px; height: 22px; }

/* Accent-tinted primary action — pairs with .primary-btn from ui.css */
.analyze-btn {
  color: var(--color-accent-text);
  background: var(--color-accent-bg);
  border-color: var(--color-accent-border);
}
.analyze-btn:hover:not(:disabled) {
  background: var(--color-accent);
  color: var(--color-on-accent);
}
.analyze-btn:active:not(:disabled) { transform: translateY(0.5px); }
.analyze-btn:focus-visible { outline: 2px solid var(--color-accent-border); outline-offset: 1px; }
.primary-btn.danger {
  color: var(--color-danger);
  border-color: var(--color-danger-bg);
  background: var(--color-danger-bg);
}
.primary-btn.danger:hover:not(:disabled) {
  background: var(--color-danger);
  color: var(--color-on-danger);
}

.file-pick-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  padding: 6px 14px;
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  font-weight: var(--weight-semibold);
  color: var(--color-accent-text);
  background: var(--color-accent-bg);
  border: 1px dashed var(--color-accent-border);
  cursor: pointer;
  transition: background 0.15s, color 0.15s, border-color 0.15s, transform 0.15s;
  font-family: inherit;
}
.file-pick-btn:hover:not(:disabled) {
  background: var(--color-accent);
  color: var(--color-on-accent);
  border-style: solid;
}
.file-pick-btn:active:not(:disabled) { transform: translateY(0.5px); }
.file-pick-btn:focus-visible { outline: 2px solid var(--color-accent-border); outline-offset: 1px; }
.file-pick-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.file-row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: var(--space-2) 10px;
}
.webdav-pick-row {
  display: flex;
  align-items: center;
  gap: var(--space-1);
}
.webdav-select {
  flex: 1;
  min-width: 0;
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  color: var(--color-text);
  font-size: var(--text-base);
  font-family: inherit;
  padding: 7px 10px;
  outline: none;
  cursor: pointer;
  transition: border-color 0.15s, box-shadow 0.15s;
}
.webdav-select:hover:not(:disabled) {
  border-color: var(--color-border-hover);
}
.webdav-select:focus,
.webdav-select:focus-visible {
  border-color: var(--color-accent-border);
  box-shadow: 0 0 0 2px var(--color-accent-bg);
}
.webdav-select:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.file-icon {
  color: var(--color-accent-text);
  flex-shrink: 0;
}
.file-name {
  flex: 1;
  font-size: 11.5px;
  font-weight: var(--weight-medium);
  color: var(--color-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* Destructive confirm — uses shared .warn-row spec from ui.css conceptually.
   Kept as a stacked column variant for this flow. */
.confirm-warn-row {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 11px var(--space-3);
  border-radius: var(--radius-md);
  background: var(--color-danger-bg);
  border-left: 3px solid var(--color-danger);
}
.confirm-warn-text {
  display: flex;
  align-items: flex-start;
  gap: var(--space-2);
  font-size: var(--text-sm);
  font-weight: var(--weight-semibold);
  color: var(--color-danger);
  line-height: 1.45;
}
.confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-1);
}
.confirm-with-countdown {
  display: flex;
  align-items: center;
  gap: var(--space-1);
}
</style>
