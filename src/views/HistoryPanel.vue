<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ArrowLeft, History, Trash2 } from "@lucide/vue";
import { useSettingsWindow } from "../composables/useSettingsWindow";
import { historyStore, loadHistory, clearAllHistory } from "../stores/config";
import { useI18n } from "vue-i18n";

const { t } = useI18n();
const router = useRouter();
const { growAbove } = useSettingsWindow(560, 380);

const showClearConfirm = ref(false);

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

async function handleClear() {
  await clearAllHistory();
  showClearConfirm.value = false;
}

onMounted(async () => {
  await loadHistory();
});

function formatTime(ts: number): string {
  const d = new Date(ts);
  const now = new Date();
  const isToday = d.toDateString() === now.toDateString();
  if (isToday) {
    return d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
  }
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
}
</script>

<template>
  <div
    class="history-root"
    :class="{ 'grow-above': growAbove }"
    @mousedown="handleDrag"
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
          class="clear-btn"
          @click.stop="showClearConfirm = true"
          :title="t('history.clearAll')"
        >
          <Trash2 :size="13" :stroke-width="1.8" />
        </button>
        <template v-if="showClearConfirm">
          <button class="confirm-clear-btn" @click.stop="handleClear">
            {{ t('common.confirm') }}
          </button>
          <button class="cancel-clear-btn" @click.stop="showClearConfirm = false">
            {{ t('common.cancel') }}
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
        <button
          v-for="(entry, i) in historyStore.entries"
          :key="entry.timestamp"
          class="history-item"
          @click="selectEntry(entry)"
        >
          <div class="history-item-input">{{ entry.input }}</div>
          <div class="history-item-output">{{ entry.output }}</div>
          <span class="history-item-time">{{ formatTime(entry.timestamp) }}</span>
        </button>
      </div>
    </main>
  </div>
</template>

<style scoped>
.history-root {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: var(--color-surface);
}
.history-root.grow-above {
  justify-content: flex-end;
}

/* Header */
.history-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  flex-shrink: 0;
  border-bottom: 1px solid var(--color-border);
}
.back-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 7px;
  border: none;
  background: transparent;
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}
.back-btn:hover { background: var(--color-surface-hover); color: var(--color-text); }

.header-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  font-weight: 600;
  color: var(--color-text);
  margin: 0;
  flex: 1;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}
.clear-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}
.clear-btn:hover { background: var(--color-danger-bg, rgba(239,68,68,0.1)); color: var(--color-danger, #ef4444); }

.confirm-clear-btn,
.cancel-clear-btn {
  font-size: 11px;
  padding: 3px 8px;
  border-radius: 5px;
  border: none;
  cursor: pointer;
  transition: background 0.15s;
}
.confirm-clear-btn {
  background: var(--color-danger, #ef4444);
  color: #fff;
}
.cancel-clear-btn {
  background: var(--color-surface-hover);
  color: var(--color-text-secondary);
}

/* Body */
.history-body {
  flex: 1;
  overflow-y: auto;
  padding: 8px 10px;
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
  flex-direction: column;
  gap: 2px;
  padding: 8px 10px;
  border-radius: 8px;
  border: 1px solid transparent;
  background: transparent;
  cursor: pointer;
  text-align: left;
  width: 100%;
  transition: background 0.12s, border-color 0.12s;
}
.history-item:hover {
  background: var(--color-surface-hover);
  border-color: var(--color-border);
}
.history-item-input {
  font-size: 12px;
  font-weight: 500;
  color: var(--color-text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.history-item-output {
  font-size: 11px;
  color: var(--color-text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.history-item-time {
  font-size: 10px;
  color: var(--color-text-tertiary, var(--color-text-secondary));
  opacity: 0.6;
  margin-top: 2px;
}
</style>
