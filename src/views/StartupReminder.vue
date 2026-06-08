<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { useShortcutTriggered } from "../composables/useTauriEvents";
import { isDark } from "../composables/useTheme";
import { X } from "@lucide/vue";

const { t } = useI18n();

const shortcutLabel = ref("...");
const countdown = ref(10);
let timer: ReturnType<typeof setInterval> | null = null;

function close() {
  if (timer) {
    clearInterval(timer);
    timer = null;
  }
  invoke("hide_main_window");
}

useShortcutTriggered(() => {
  close();
});

onMounted(async () => {
  shortcutLabel.value = await invoke<string>("get_shortcut_label");
  invoke("show_startup_reminder_window");

  timer = setInterval(() => {
    countdown.value--;
    if (countdown.value <= 0) {
      close();
    }
  }, 1000);
});

onUnmounted(() => {
  if (timer) {
    clearInterval(timer);
  }
});
</script>

<template>
  <div class="reminder-root" :class="{ dark: isDark() }">
    <div class="reminder-card">
      <!-- Close button with countdown -->
      <button class="close-btn" @click="close" :title="t('common.hide')">
        <X :size="14" :stroke-width="2" />
        <span class="countdown-text">{{ countdown }}</span>
      </button>

      <!-- Content -->
      <img class="reminder-logo" src="/prompit_logo.svg" alt="" />
      <span class="reminder-name">Prompit</span>
      <p class="reminder-hint">{{ t('startupReminder.hint', { shortcut: shortcutLabel }) }}</p>
    </div>
  </div>
</template>

<style scoped>
.reminder-root {
  height: 100dvh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--color-bg);
  color: var(--color-text);
  overflow: hidden;
  border-radius: 11px;
}

.reminder-card {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 32px 40px 28px;
}

.close-btn {
  position: absolute;
  top: -4px;
  right: -12px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 3px;
  width: 28px;
  height: 28px;
  border-radius: 7px;
  border: none;
  background: transparent;
  color: var(--color-text-muted);
  cursor: pointer;
  transition: 0.15s;
}

.close-btn:hover {
  color: var(--color-text);
  background: var(--color-surface-hover);
}

.countdown-text {
  font-size: 10px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  color: var(--color-text-muted);
}

.reminder-logo {
  height: 2em;
  width: auto;
  margin-bottom: 4px;
}

.reminder-name {
  font-size: 16px;
  font-weight: 700;
  letter-spacing: -0.02em;
  color: var(--color-text);
}

.reminder-hint {
  margin-top: 8px;
  font-size: 13px;
  color: var(--color-text-secondary);
  text-align: center;
}
</style>
