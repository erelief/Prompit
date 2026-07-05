<script setup lang="ts">
import { computed, ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useIntervalFn } from "@vueuse/core";
import { useSettingsWindow } from "../composables/useSettingsWindow";
import DataCategorySelector from "../components/DataCategorySelector.vue";
import { ALL_CATEGORIES } from "../composables/useDataCategories";
import {
  loadConfig, loadHistory, loadPersonas, loadSkillsLites, refreshDictStatus,
} from "../stores/config";
import { ArrowLeft, ShieldAlert, Check, X } from "@lucide/vue";

const { t } = useI18n();
const router = useRouter();
const { growAbove } = useSettingsWindow();

const selected = ref<string[]>([...ALL_CATEGORIES]);
const status = ref<{ kind: "idle" | "success" | "error"; msg: string }>({ kind: "idle", msg: "" });

const countdown = ref(5);
const ready = ref(false);
const isSandbox = ref(false);
const busy = ref(false);

const isFullReset = computed(() => selected.value.length >= ALL_CATEGORIES.length);
const includesSettings = computed(() => selected.value.includes("settings"));

const { pause } = useIntervalFn(() => {
  countdown.value--;
  if (countdown.value <= 0) {
    ready.value = true;
    pause();
  }
}, 1000);

onMounted(async () => {
  isSandbox.value = await invoke<boolean>("is_sandbox");
});

async function handleDrag(e: MouseEvent) {
  const target = e.target as HTMLElement;
  if (target.closest("button, input, textarea, a, select, .reset-footer, .cat-selector")) return;
  await getCurrentWindow().startDragging();
}

function cancel() {
  router.push("/settings?tab=general");
}

async function handleConfirm() {
  if (busy.value || !ready.value) return;
  busy.value = true;
  try {
    if (isFullReset.value) {
      // Nuclear path: wipe the whole data dir and exit, same as before.
      await invoke("reset_app_data");
      return;
    }
    // Partial path: delete only the selected on-disk files, then hot-reload.
    await invoke("delete_categories", { categories: selected.value });
    if (includesSettings.value) {
      // Settings file was deleted — the in-memory config is now stale and the
      // app will re-create defaults on next launch. Surface a restart hint.
      status.value = {
        kind: "success",
        msg: t("settings.reset.restartRequired"),
      };
    } else {
      await loadConfig();
      await Promise.all([
        loadHistory(),
        loadPersonas(),
        loadSkillsLites(),
        refreshDictStatus(),
      ]).catch(() => {});
      status.value = {
        kind: "success",
        msg: t("settings.reset.partialSuccess"),
      };
    }
  } catch (err) {
    status.value = { kind: "error", msg: String(err) };
    console.error("Failed to delete categories:", err);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div class="reset-root" :class="{ 'grow-above': growAbove }" @mousedown="handleDrag">
    <!-- Header -->
    <div class="reset-header">
      <button class="back-btn" @click="router.push('/settings?tab=general')">
        <ArrowLeft :size="16" />
      </button>
      <span class="header-title">{{ t('settings.reset.pageTitle') }}</span>
      <span v-if="isSandbox" class="sandbox-badge">{{ t('settings.reset.sandboxBadge') }}</span>
    </div>

    <!-- Body -->
    <div class="reset-body">
      <div class="warn-card">
        <div class="warn-icon-wrap">
          <ShieldAlert :size="22" :stroke-width="1.5" />
        </div>
        <div class="warn-content">
          <p class="warn-text">{{ isFullReset ? t('settings.reset.warning') : t('settings.reset.partialWarning') }}</p>
          <p class="warn-irreversible">{{ t('settings.reset.irreversible') }}</p>
        </div>
      </div>
      <p v-if="isSandbox" class="sandbox-hint">{{ t('settings.reset.sandboxHint') }}</p>

      <div class="selector-label">{{ t('settings.reset.selectCategories') }}</div>
      <DataCategorySelector v-model="selected" :available="ALL_CATEGORIES" :disabled="busy" />
    </div>

    <!-- Footer -->
    <div class="reset-footer">
      <span class="understood-label">{{ t('settings.reset.understood') }}</span>
      <span
        v-if="status.kind !== 'idle'"
        class="status-text"
        :class="{ success: status.kind === 'success', error: status.kind === 'error' }"
      >{{ status.msg }}</span>
      <div class="footer-actions">
        <button class="mini-btn" :title="t('common.cancel')" :disabled="busy" @click="cancel">
          <X :size="12" :stroke-width="2.5" />
        </button>
        <div class="confirm-with-countdown">
          <button
            class="mini-btn danger-active"
            :class="{ 'confirm-counting': !ready || busy }"
            :title="ready && !busy ? t('common.confirm') : t('settings.reset.confirmCountdown', { n: countdown })"
            :disabled="!ready || busy"
            @click="handleConfirm"
          >
            <Check :size="12" :stroke-width="2.5" />
          </button>
          <span v-if="!ready" class="countdown-label">{{ countdown }}s</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.reset-root {
  height: calc(100dvh / var(--font-scale, 1));
  display: flex;
  flex-direction: column;
  background: var(--color-bg);
  color: var(--color-text);
  overflow: hidden;
  border-radius: 11px;
}
.reset-root.grow-above .reset-header { order: 2; border-bottom: none; border-top: 1px solid var(--color-surface); }
.reset-root.grow-above .reset-footer { order: 1; border-top: none; border-bottom: 1px solid var(--color-surface); }
.reset-root.grow-above .reset-body { order: 0; }

.reset-header {
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
.sandbox-badge {
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.04em;
  padding: 2px 8px;
  border-radius: 6px;
  background: rgba(34, 197, 94, 0.12);
  color: #16a34a;
  flex-shrink: 0;
}

.reset-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: stretch;
  padding: 18px 24px;
  overflow-y: auto;
  gap: 12px;
}
.reset-body::-webkit-scrollbar { width: 3px; }
.reset-body::-webkit-scrollbar-thumb { background: var(--color-scrollbar); border-radius: 3px; }
.warn-card {
  display: flex;
  align-items: flex-start;
  gap: 16px;
}
.warn-icon-wrap {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  border-radius: 11px;
  background: var(--color-danger-bg);
  color: var(--color-danger);
  flex-shrink: 0;
}
.warn-content {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-top: 2px;
}
.warn-text {
  font-size: 13px;
  font-weight: 500;
  line-height: 1.55;
  color: var(--color-text-secondary);
}
.warn-irreversible {
  font-size: 11px;
  font-weight: 650;
  color: var(--color-danger);
  letter-spacing: 0.01em;
}
.sandbox-hint {
  font-size: 11px;
  font-weight: 500;
  color: #16a34a;
}
.selector-label {
  font-size: 10.5px;
  font-weight: 650;
  color: var(--color-text-secondary);
  letter-spacing: 0.01em;
}

.reset-footer {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 24px 16px;
  border-top: 1px solid var(--color-surface);
  flex-shrink: 0;
}
.understood-label {
  font-size: 11px;
  font-weight: 500;
  color: var(--color-text-muted);
  flex-shrink: 0;
}
.status-text {
  font-size: 10.5px;
  font-weight: 500;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.status-text.success { color: var(--color-success); }
.status-text.error { color: var(--color-danger); }
.footer-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-left: auto;
}

.mini-btn {
  display: flex; align-items: center; justify-content: center;
  width: 27px; height: 27px; border-radius: 7px;
  color: var(--color-text-muted); cursor: pointer;
  border: none; background: none; transition: .12s;
}
.mini-btn:hover:not(:disabled) { color: var(--color-text); background: var(--color-border); }
.mini-btn.danger-active {
  color: var(--color-danger); background: var(--color-danger-bg);
  animation: danger-pulse .8s ease-in-out infinite alternate;
}
.mini-btn.confirm-counting {
  opacity: .55;
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
  opacity: .85;
  min-width: 20px;
}
@keyframes danger-pulse {
  to { background: var(--color-danger-bg); filter: brightness(.88); }
}
</style>
