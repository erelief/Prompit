<script setup lang="ts">
import { ref, computed } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useIntervalFn } from "@vueuse/core";
import { useSettingsWindow } from "../composables/useSettingsWindow";
import {
  ArrowLeft, Download, Upload, Eye, EyeOff, Check, X,
  ShieldAlert, FileText, FolderOpen, Lock,
} from "@lucide/vue";

const { t } = useI18n();
const router = useRouter();
const { growAbove } = useSettingsWindow();

// ── Export state ──
const exportPassword = ref("");
const exportShowPw = ref(false);
const exportConfirmed = ref(false);
const exportStatus = ref<{ kind: "idle" | "info" | "success" | "error"; msg: string }>({ kind: "idle", msg: "" });
const exportBusy = ref(false);

const exportCanConfirm = computed(() => exportPassword.value.length >= 6 && !exportConfirmed.value);

function confirmExportPassword() {
  if (!exportCanConfirm.value) return;
  exportConfirmed.value = true;
}

function resetExport() {
  exportPassword.value = "";
  exportConfirmed.value = false;
  exportShowPw.value = false;
}

async function handleExport() {
  if (!exportConfirmed.value) return;
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
    resetExport();
  } catch (err) {
    exportStatus.value = { kind: "error", msg: t("settings.userData.error", { message: String(err) }) };
  } finally {
    exportBusy.value = false;
  }
}

// ── Import state ──
const importPath = ref<string | null>(null);
const importPassword = ref("");
const importShowPw = ref(false);
const importConfirming = ref(false);
const importCountdown = ref(5);
const importStatus = ref<{ kind: "idle" | "info" | "success" | "error"; msg: string }>({ kind: "idle", msg: "" });
const importBusy = ref(false);

const importTimer = useIntervalFn(() => {
  if (importCountdown.value > 0) importCountdown.value--;
  else importTimer.pause();
}, 1000, { immediate: false });

const importFileName = computed(() => {
  if (!importPath.value) return "";
  const parts = importPath.value.replace(/\\/g, "/").split("/");
  return parts[parts.length - 1] || importPath.value;
});

const importCanConfirm = computed(() => !!importPath.value && importPassword.value.length > 0 && !importConfirming.value);

async function selectImportFile() {
  const selected = await open({
    multiple: false,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  const path = typeof selected === "string" ? selected : null;
  if (!path) {
    importStatus.value = { kind: "info", msg: t("settings.userData.import.cancelled") };
    return;
  }
  importPath.value = path;
  importPassword.value = "";
  importConfirming.value = false;
  importCountdown.value = 5;
  importTimer.pause();
  importStatus.value = { kind: "idle", msg: "" };
}

function resetImport() {
  importPath.value = null;
  importPassword.value = "";
  importShowPw.value = false;
  importConfirming.value = false;
  importCountdown.value = 5;
  importTimer.pause();
}

function requestImport() {
  if (!importCanConfirm.value) return;
  importConfirming.value = true;
  importCountdown.value = 5;
  importTimer.resume();
}

function cancelImportConfirm() {
  importConfirming.value = false;
  importCountdown.value = 5;
  importTimer.pause();
}

async function confirmImport() {
  if (importCountdown.value > 0 || !importPath.value) return;
  importBusy.value = true;
  try {
    await invoke("import_data", { path: importPath.value, password: importPassword.value });
    importStatus.value = { kind: "success", msg: t("settings.userData.import.success") };
    resetImport();
  } catch (err) {
    importStatus.value = { kind: "error", msg: t("settings.userData.error", { message: String(err) }) };
    cancelImportConfirm();
  } finally {
    importBusy.value = false;
  }
}

// ── Shared ──
async function handleDrag(e: MouseEvent) {
  const target = e.target as HTMLElement;
  if (target.closest("button, input, textarea, a, select, .ud-card, .ud-footer")) return;
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
      <span class="header-title">{{ t('settings.userData.pageTitle') }}</span>
    </div>

    <!-- Body -->
    <div class="ud-body settings-scrollbar">
      <!-- ═══ Export ═══ -->
      <section class="ud-card export-card">
        <div class="ud-card-head">
          <Download :size="13" />
          <span class="ud-card-title">{{ t('settings.userData.export.title') }}</span>
        </div>
        <p class="ud-desc">{{ t('settings.userData.export.description') }}</p>

        <!-- password + confirm + export -->
        <div class="pw-row" :class="{ locked: exportConfirmed }">
          <Lock v-if="exportConfirmed" :size="12" class="pw-lock-icon" />
          <input
            :type="exportShowPw ? 'text' : 'password'"
            class="pw-input"
            v-model="exportPassword"
            :placeholder="t('settings.userData.export.passwordPlaceholder')"
            :readonly="exportConfirmed"
            autocomplete="new-password"
            @input="exportConfirmed = false"
          />
          <button v-if="!exportConfirmed" class="pw-toggle" @click="exportShowPw = !exportShowPw" type="button">
            <Eye v-if="!exportShowPw" :size="13" />
            <EyeOff v-else :size="13" />
          </button>
        </div>

        <div class="action-row">
          <!-- Confirm button: gates the Export -->
          <button
            v-if="!exportConfirmed"
            class="ud-btn confirm-btn"
            :class="{ active: exportCanConfirm }"
            :disabled="!exportCanConfirm"
            @click="confirmExportPassword"
          >
            <Check :size="12" :stroke-width="2" />{{ t('settings.userData.export.confirm') }}
          </button>
          <button
            v-else
            class="ud-btn confirmed-btn"
            disabled
          >
            <Check :size="12" :stroke-width="2.5" />{{ t('settings.userData.export.confirmed') }}
          </button>

          <!-- Export button: enabled only after confirm -->
          <button
            class="ud-btn primary-btn"
            :disabled="!exportConfirmed || exportBusy"
            @click="handleExport"
          >
            <Download :size="12" :stroke-width="1.9" />{{ t('settings.userData.export.button') }}
          </button>
        </div>
        <p v-if="!exportConfirmed" class="ud-hint">{{ t('settings.userData.export.hint') }}</p>
      </section>

      <!-- ═══ Divider ═══ -->
      <div class="ud-divider"><span>{{ t('common.or') }}</span></div>

      <!-- ═══ Import ═══ -->
      <section class="ud-card import-card">
        <div class="ud-card-head">
          <Upload :size="13" />
          <span class="ud-card-title">{{ t('settings.userData.import.title') }}</span>
        </div>
        <p class="ud-desc">{{ t('settings.userData.import.description') }}</p>
        <p class="ud-warn">{{ t('settings.userData.import.warning') }}</p>

        <!-- Step 1: select file -->
        <button
          v-if="!importPath"
          class="file-pick-btn"
          @click="selectImportFile"
        >
          <FolderOpen :size="14" :stroke-width="1.8" />{{ t('settings.userData.import.selectFile') }}
        </button>

        <!-- Step 2/3: file selected -->
        <template v-else>
          <!-- selected file row -->
          <div class="file-row">
            <FileText :size="14" class="file-icon" />
            <span class="file-name" :title="importPath">{{ importFileName }}</span>
            <button
              class="pw-toggle change-file-btn"
              :disabled="importConfirming || importBusy"
              @click="selectImportFile"
              type="button"
            >
              <FolderOpen :size="13" />
            </button>
          </div>

          <!-- password (hidden while confirming) -->
          <div v-if="!importConfirming" class="pw-row">
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

          <!-- Step 2: import button -->
          <button
            v-if="!importConfirming"
            class="ud-btn primary-btn danger"
            :disabled="!importCanConfirm"
            @click="requestImport"
          >
            <Upload :size="12" :stroke-width="1.9" />{{ t('settings.userData.import.button') }}
          </button>

          <!-- Step 3: countdown confirm -->
          <div v-else class="confirm-warn-row">
            <div class="confirm-warn-text">
              <ShieldAlert :size="14" :stroke-width="1.6" />
              <span>{{ t('settings.userData.import.confirmWarning') }}</span>
            </div>
            <div class="confirm-actions">
              <button class="mini-btn" :title="t('common.cancel')" :disabled="importBusy" @click="cancelImportConfirm">
                <X :size="12" :stroke-width="2.5" />
              </button>
              <div class="confirm-with-countdown">
                <button
                  class="mini-btn danger-active"
                  :class="{ 'confirm-counting': importCountdown > 0 }"
                  :title="importCountdown > 0 ? t('settings.reset.confirmCountdown', { n: importCountdown }) : t('common.confirm')"
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
      </section>
    </div>

    <!-- Footer status -->
    <div class="ud-footer">
      <span
        v-if="exportStatus.kind !== 'idle'"
        class="status-text"
        :class="{ success: exportStatus.kind === 'success', error: exportStatus.kind === 'error', info: exportStatus.kind === 'info' }"
      >{{ exportStatus.msg }}</span>
      <span
        v-if="importStatus.kind !== 'idle'"
        class="status-text"
        :class="{ success: importStatus.kind === 'success', error: importStatus.kind === 'error', info: importStatus.kind === 'info' }"
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

/* ── Header ── */
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

/* ── Body ── */
.ud-body {
  flex: 1;
  overflow-y: auto;
  padding: 18px 24px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

/* ── Section card ── */
.ud-card {
  display: flex;
  flex-direction: column;
  gap: 9px;
  background: var(--color-surface);
  border: 1px solid var(--color-scrollbar);
  border-radius: 10px;
  padding: 14px;
}
.ud-card-head {
  display: flex;
  align-items: center;
  gap: 7px;
  color: var(--color-text-secondary);
}
.ud-card-title {
  font-size: 12px;
  font-weight: 650;
  letter-spacing: 0.01em;
}
.ud-desc {
  font-size: 11px;
  font-weight: 500;
  line-height: 1.55;
  color: var(--color-text-muted);
}
.ud-warn {
  font-size: 10.5px;
  font-weight: 600;
  color: var(--color-danger);
  letter-spacing: 0.01em;
}
.ud-hint {
  font-size: 10px;
  font-weight: 500;
  color: var(--color-text-muted);
  opacity: 0.8;
}

/* ── Divider between sections ── */
.ud-divider {
  display: flex;
  align-items: center;
  gap: 10px;
  margin: 2px 0;
  color: var(--color-text-muted);
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.ud-divider::before,
.ud-divider::after {
  content: "";
  flex: 1;
  height: 1px;
  background: var(--color-border);
}

/* ── Password input ── */
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
.pw-row.locked {
  background: var(--color-surface-hover);
  border-color: var(--color-scrollbar);
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
.pw-input:read-only { cursor: default; }
.pw-lock-icon {
  color: var(--color-success);
  flex-shrink: 0;
}
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
.pw-toggle:disabled { opacity: 0.4; cursor: default; }

/* ── Action buttons ── */
.action-row {
  display: flex;
  gap: 8px;
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

/* Confirm button: muted → accent when active */
.confirm-btn {
  color: var(--color-text-muted);
  border-color: var(--color-border);
  background: var(--color-bg);
}
.confirm-btn.active {
  color: var(--color-accent-text);
  border-color: var(--color-accent-border);
  background: var(--color-accent-bg);
}
.confirm-btn.active:hover:not(:disabled) {
  background: var(--color-accent);
  color: var(--color-bg);
}

/* Confirmed state: green */
.confirmed-btn {
  color: var(--color-success);
  border-color: var(--color-success);
  background: var(--color-success-bg);
  cursor: default;
}

/* Primary (Export/Import) button */
.primary-btn {
  color: var(--color-text);
  background: var(--color-surface-hover);
  border-color: var(--color-border);
  flex: 1;
}
.primary-btn:hover:not(:disabled) {
  background: var(--color-border);
}
.primary-btn.danger {
  color: var(--color-danger);
  border-color: var(--color-danger-bg);
  background: var(--color-danger-bg);
}
.primary-btn.danger:hover:not(:disabled) {
  background: var(--color-danger);
  color: var(--color-bg);
}

/* ── File picker (step 1) ── */
.file-pick-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  padding: 10px 14px;
  border-radius: 8px;
  font-size: 11.5px;
  font-weight: 600;
  color: var(--color-accent-text);
  background: var(--color-accent-bg);
  border: 1px dashed var(--color-accent-border);
  cursor: pointer;
  transition: 0.15s;
  font-family: inherit;
}
.file-pick-btn:hover {
  background: var(--color-accent);
  color: var(--color-bg);
  border-style: solid;
}

/* ── Selected file row ── */
.file-row {
  display: flex;
  align-items: center;
  gap: 8px;
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: 8px;
  padding: 8px 10px;
}
.file-icon {
  color: var(--color-accent-text);
  flex-shrink: 0;
}
.file-name {
  flex: 1;
  font-size: 11.5px;
  font-weight: 500;
  color: var(--color-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.change-file-btn {
  width: 22px;
  height: 22px;
}

/* ── Danger countdown confirm ── */
.confirm-warn-row {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 11px 12px;
  border-radius: 8px;
  background: var(--color-danger-bg);
  border-left: 3px solid var(--color-danger);
}
.confirm-warn-text {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  font-size: 11px;
  font-weight: 600;
  color: var(--color-danger);
  line-height: 1.45;
}
.confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: 6px;
}
.mini-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 27px;
  height: 27px;
  border-radius: 7px;
  color: var(--color-text-muted);
  cursor: pointer;
  border: none;
  background: none;
  transition: 0.12s;
}
.mini-btn:hover:not(:disabled) {
  color: var(--color-text);
  background: var(--color-border);
}
.mini-btn.danger-active {
  color: var(--color-danger);
  background: var(--color-bg);
  animation: danger-pulse 0.8s ease-in-out infinite alternate;
}
.mini-btn.confirm-counting {
  opacity: 0.55;
  cursor: not-allowed;
  animation: none;
  color: var(--color-text-muted);
  background: var(--color-surface);
}
.confirm-with-countdown {
  display: flex;
  align-items: center;
  gap: 4px;
}
.countdown-label {
  font-size: 10px;
  font-weight: 600;
  color: var(--color-text-muted);
  font-variant-numeric: tabular-nums;
  opacity: 0.85;
  min-width: 20px;
}
@keyframes danger-pulse {
  to { filter: brightness(0.88); }
}

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
.status-text.success { color: var(--color-success); }
.status-text.error { color: var(--color-danger); }
.status-text.info { color: var(--color-text-muted); }
</style>
