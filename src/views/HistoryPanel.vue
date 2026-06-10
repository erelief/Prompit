<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRouter } from "vue-router";

import { getCurrentWindow } from "@tauri-apps/api/window";
import { ArrowLeft, History, Trash2, Check, X, Send, MessageSquare } from "@lucide/vue";
import { useSettingsWindow } from "../composables/useSettingsWindow";
import { appConfig, historyStore, loadHistory, clearAllHistory, saveHistory } from "../stores/config";
import { isDark } from "../composables/useTheme";
import { useI18n } from "vue-i18n";

const { t } = useI18n();
const router = useRouter();
const { growAbove } = useSettingsWindow();

const glassBg = computed(() => {
  const o = (appConfig.floating_opacity ?? 90) / 100;
  if (o >= 1) {
    return isDark()
      ? "linear-gradient(135deg, rgb(15,15,20) 0%, rgb(20,20,30) 100%)"
      : "linear-gradient(135deg, rgb(255,255,255) 0%, rgb(245,245,250) 100%)";
  }
  return isDark()
    ? `linear-gradient(135deg, rgba(15,15,20,${o}) 0%, rgba(20,20,30,${o * 0.94}) 100%)`
    : `linear-gradient(135deg, rgba(255,255,255,${o}) 0%, rgba(245,245,250,${o * 0.94}) 100%)`;
});

const showClearConfirm = ref(false);
const pendingRemove = ref<number | null>(null);

async function goBack() {
  router.push("/");
}

async function handleDrag(e: MouseEvent) {
  const target = e.target as HTMLElement;
  if (target.closest("button, a, .history-item")) return;
  await getCurrentWindow().startDragging();
}

function selectEntry(entry: { input: string; output: string }) {
  sessionStorage.setItem("history-restore", JSON.stringify(entry));
  router.push("/");
}

function shortModel(model: string): string {
  // strip date suffix like "-2024-07-18"
  let s = model.replace(/-\d{4}-\d{2}-\d{2}$/, "");
  // strip common provider prefixes
  s = s.replace(/^(openai|anthropic|google|deepseek|zhipu|minimax)-/i, "");
  return s.length > 14 ? s.slice(0, 12) + "…" : s;
}

async function handleClear() {
  await clearAllHistory();
  showClearConfirm.value = false;
}

function requestRemove(index: number) {
  pendingRemove.value = index;
}
function cancelRemove() {
  pendingRemove.value = null;
}
async function confirmRemove(index: number) {
  historyStore.entries.splice(index, 1);
  pendingRemove.value = null;
  await saveHistory();
}

onMounted(async () => {
  await loadHistory();
});
</script>

<template>
  <div
    class="history-root"
    :class="{ 'grow-above': growAbove }"
    @mousedown="handleDrag"
    :style="{ background: glassBg, backdropFilter: 'blur(24px) saturate(1.5)' }"
  >
    <!-- Header -->
    <header class="history-header">
      <button @click="goBack" class="back-btn" :title="t('common.settings')">
        <ArrowLeft :size="18" :stroke-width="1.8" />
      </button>
      <h1 class="header-title">
        <History :size="14" :stroke-width="1.8" />
        {{ t('history.title') }}
      </h1>
      <div class="header-actions">
        <button
          v-if="!showClearConfirm && historyStore.entries.length > 0"
          class="reset-btn"
          @click.stop="showClearConfirm = true"
          :title="t('history.clearAll')"
        >
          <Trash2 :size="11" :stroke-width="1.9" />{{ t('history.clearAll') }}
        </button>
        <template v-if="showClearConfirm">
          <button class="mini-btn danger-active" :title="t('common.confirm')" @click.stop="handleClear">
            <Check :size="11" :stroke-width="2.5" />
          </button>
          <button class="mini-btn" :title="t('common.cancel')" @click.stop="showClearConfirm = false">
            <X :size="11" :stroke-width="2.5" />
          </button>
        </template>
      </div>
    </header>

    <!-- List -->
    <main class="history-body">
      <div v-if="historyStore.entries.length === 0" class="empty-state">
        <History :size="28" :stroke-width="1" class="empty-icon" />
        <span class="empty-text">{{ t('history.empty') }}<br><small>{{ t('history.emptySub') }}</small></span>
      </div>
      <div v-else class="history-list">
        <div
          v-for="(entry, idx) in historyStore.entries"
          :key="entry.timestamp"
          class="history-item"
          :class="{ 'remove-pending': pendingRemove === idx }"
        >
          <template v-if="pendingRemove === idx">
            <span class="remove-warning-text">{{ t('common.cannotBeUndone') }}</span>
            <div class="remove-actions">
              <button class="mini-btn danger-active" :title="t('common.confirm')" @click.stop="confirmRemove(idx)">
                <Check :size="11" :stroke-width="2.5" />
              </button>
              <button class="mini-btn" :title="t('common.cancel')" @click.stop="cancelRemove">
                <X :size="11" :stroke-width="2.5" />
              </button>
            </div>
          </template>
          <template v-else>
            <button class="history-item-main" @click="selectEntry(entry)">
              <div class="history-item-input">
                <Send :size="9" :stroke-width="2" class="input-icon" />
                <span>{{ entry.input }}</span>
              </div>
              <div class="history-item-output">
                <MessageSquare :size="9" :stroke-width="2" class="output-icon" />
                <span>{{ entry.output }}</span>
                <span v-if="entry.model" class="model-badge">{{ shortModel(entry.model) }}</span>
              </div>
            </button>
            <div class="history-item-actions" @click.stop>
              <button class="mini-btn warn" :title="t('common.remove')" @click="requestRemove(idx)">
                <Trash2 :size="11" :stroke-width="1.9" />
              </button>
            </div>
          </template>
        </div>
        <div class="history-footer">
          {{ t('history.entryCount', { current: historyStore.entries.length, limit: appConfig.history_limit || 50 }) }}
        </div>
      </div>
    </main>
  </div>
</template>

<style scoped>
.history-root {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100dvh;
  overflow: hidden;
}
.history-root.grow-above {
  flex-direction: column-reverse;
}
.history-root.grow-above .history-header {
  border-bottom: none;
  border-top: 1px solid var(--color-surface);
}

/* Header */
.history-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px 24px 12px;
  flex-shrink: 0;
  border-bottom: 1px solid var(--color-surface);
}
.back-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 9px;
  border: none;
  background: transparent;
  color: var(--color-text-muted);
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}
.back-btn:hover { background: var(--color-surface-hover); color: var(--color-text); }

.header-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 15px;
  font-weight: 700;
  letter-spacing: -0.02em;
  color: var(--color-text);
  line-height: 1.2;
  margin: 0;
  flex: 1;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}
.reset-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 8px;
  border-radius: 7px;
  font-size: 10.5px;
  font-weight: 550;
  color: var(--color-text-muted);
  cursor: pointer;
  border: none;
  background: none;
  transition: 0.15s;
}
.reset-btn:hover {
  color: var(--color-danger);
  background: var(--color-danger-bg);
}

.mini-btn {
  display: flex; align-items: center; justify-content: center;
  width: 27px; height: 27px; border-radius: 7px;
  color: var(--color-text-muted); cursor: pointer;
  border: none; background: none; transition: 0.12s;
}
.mini-btn:hover { color: var(--color-text); background: var(--color-border); }
.mini-btn.warn:hover { color: var(--color-danger); background: var(--color-danger-bg); }
.mini-btn.danger-active {
  color: var(--color-danger); background: var(--color-danger-bg);
  animation: danger-pulse 0.8s ease-in-out infinite alternate;
}
@keyframes danger-pulse {
  to { background: var(--color-danger-bg); filter: brightness(.88); }
}

/* Body */
.history-body {
  flex: 1;
  overflow-y: auto;
  padding: 10px 24px 16px;
}
.history-body::-webkit-scrollbar { width: 3px; }
.history-body::-webkit-scrollbar-thumb { background: var(--color-scrollbar); border-radius: 3px; }

/* Empty */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 40px 20px;
  color: var(--color-text-secondary);
  text-align: center;
  font-size: 12px;
  line-height: 1.5;
}
.empty-icon { opacity: 0.4; }

/* List items */
.history-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.history-item {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 8px 10px;
  border-radius: 8px;
  border: 1px solid transparent;
  background: transparent;
  transition: background 0.12s, border-color 0.12s;
}
.history-item:hover {
  background: var(--color-surface-hover);
  border-color: var(--color-border);
}
.history-item.remove-pending {
  background: var(--color-danger-bg);
  border-color: var(--color-border);
  justify-content: space-between;
}
.history-item-main {
  display: flex;
  flex-direction: column;
  gap: 3px;
  cursor: pointer;
  text-align: left;
  width: 100%;
  background: none;
  border: none;
  padding: 0;
  min-width: 0;
}
.history-item-input {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
  font-weight: 550;
  color: var(--color-text);
  line-height: 1.3;
}
.history-item-input span {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.history-item-input .input-icon {
  flex-shrink: 0;
  color: var(--color-accent);
  opacity: 0.7;
}
.history-item-output {
  display: flex;
  align-items: flex-start;
  gap: 5px;
  font-size: 11px;
  color: var(--color-text-secondary);
  line-height: 1.3;
  min-width: 0;
}
.history-item-output span {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}
.model-badge {
  flex-shrink: 0;
  font-size: 9px;
  font-weight: 600;
  letter-spacing: 0.02em;
  color: var(--color-text-tertiary, var(--color-text-muted));
  background: var(--color-surface-hover);
  padding: 0 5px;
  border-radius: 4px;
  line-height: 16px;
  white-space: nowrap;
}
.history-item-output .output-icon {
  flex-shrink: 0;
  color: var(--color-text-tertiary, var(--color-text-muted));
  opacity: 0.5;
  margin-top: 1px;
}
.history-item-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  opacity: 0;
  transition: opacity 0.12s;
  flex-shrink: 0;
}
.history-item:hover .history-item-actions {
  opacity: 1;
}
.remove-warning-text {
  font-size: 10px;
  font-weight: 550;
  letter-spacing: 0.01em;
  color: var(--color-danger);
}
.remove-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  flex-shrink: 0;
}
.history-footer {
  margin-top: 8px;
  padding-top: 10px;
  border-top: 1px solid var(--color-surface);
  text-align: center;
  font-size: 10.5px;
  color: var(--color-text-tertiary, var(--color-text-muted));
  letter-spacing: 0.01em;
}
</style>
