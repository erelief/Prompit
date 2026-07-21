<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { useIntervalFn } from "@vueuse/core";
import { useShortcutTriggered } from "../composables/useTauriEvents";
import { X } from "@lucide/vue";

const { t } = useI18n();
const router = useRouter();

const shortcutLabel = ref("...");
const countdown = ref(10);

function close() {
  pause();
  router.replace("/");
  invoke("hide_main_window");
}

useShortcutTriggered(() => {
  // Shortcut pressed: transition to input view without hiding window
  pause();
  router.replace("/");
});

const { pause } = useIntervalFn(() => {
  countdown.value--;
  if (countdown.value <= 0) {
    close();
  }
}, 1000);

onMounted(async () => {
  shortcutLabel.value = await invoke<string>("get_shortcut_label");
  invoke("show_startup_reminder_window");
});
</script>

<template>
  <div class="reminder-root">
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
  height: calc(100dvh / var(--font-scale, 1));
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--color-bg);
  color: var(--color-text);
  overflow: hidden;
  border-radius: var(--radius-lg);
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
  top: var(--space-2);
  right: var(--space-2);
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 3px;
  width: 28px;
  height: 28px;
  border-radius: var(--radius-sm);
  border: none;
  background: transparent;
  color: var(--color-text-muted);
  cursor: pointer;
  transition: color 0.15s, background 0.15s;
}

.close-btn:hover {
  color: var(--color-text);
  background: var(--color-surface-hover);
}
.close-btn:active { background: var(--color-border); }
.close-btn:focus-visible { outline: 2px solid var(--color-accent-border); outline-offset: 1px; }

.countdown-text {
  font-size: var(--text-xs);
  font-weight: var(--weight-semibold);
  font-variant-numeric: tabular-nums;
  color: var(--color-text-muted);
}

.reminder-logo {
  height: 2em;
  width: auto;
  margin-bottom: var(--space-1);
}

.reminder-name {
  font-size: var(--text-lg);
  font-weight: var(--weight-bold);
  letter-spacing: -0.02em;
  color: var(--color-text);
}

.reminder-hint {
  margin-top: var(--space-2);
  font-size: var(--text-md);
  color: var(--color-text-secondary);
  text-align: center;
}
</style>
