<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { ArrowLeft, AlertTriangle } from "@lucide/vue";

const { t } = useI18n();
const router = useRouter();

const countdown = ref(5);
const ready = ref(false);
let timer: ReturnType<typeof setInterval> | null = null;

onMounted(() => {
  timer = setInterval(() => {
    countdown.value--;
    if (countdown.value <= 0) {
      ready.value = true;
      if (timer) clearInterval(timer);
    }
  }, 1000);
});

onUnmounted(() => {
  if (timer) clearInterval(timer);
});

function cancel() {
  router.push("/settings?tab=general");
}

async function handleConfirm() {
  try {
    await invoke("reset_app_data");
  } catch (err) {
    console.error("Failed to reset app data:", err);
  }
}
</script>

<template>
  <div class="reset-root">
    <!-- Header -->
    <div class="reset-header">
      <button class="back-btn" @click="router.push('/settings?tab=general')">
        <ArrowLeft :size="16" />
      </button>
      <span class="header-title">{{ t('settings.reset.pageTitle') }}</span>
    </div>

    <!-- Warning -->
    <div class="reset-body">
      <div class="warn-card">
        <div class="warn-icon-wrap">
          <AlertTriangle :size="20" :stroke-width="1.6" />
        </div>
        <p class="warn-text">{{ t('settings.reset.warning') }}</p>
        <p class="warn-sub">{{ t('settings.reset.irreversible') }}</p>
      </div>
    </div>

    <!-- Footer -->
    <div class="reset-footer">
      <span class="understood-text">{{ t('settings.reset.understood') }}</span>
      <div class="footer-actions">
        <button class="cancel-btn" @click="cancel">
          {{ t('common.cancel') }}
        </button>
        <button
          class="confirm-btn"
          :class="{ ready }"
          :disabled="!ready"
          @click="handleConfirm"
        >
          {{ ready ? t('common.confirm') : t('settings.reset.confirmCountdown', { n: countdown }) }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.reset-root {
  height: 100dvh;
  display: flex;
  flex-direction: column;
  background: var(--color-bg);
  color: var(--color-text);
  overflow: hidden;
  border-radius: 11px;
}

/* ── Header ── */
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

/* ── Body ── */
.reset-body {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
}

.warn-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
  max-width: 300px;
  text-align: center;
}
.warn-icon-wrap {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 48px;
  height: 48px;
  border-radius: 14px;
  background: var(--color-danger-bg);
  color: var(--color-danger);
}
.warn-text {
  font-size: 13px;
  font-weight: 550;
  line-height: 1.55;
  color: var(--color-text);
}
.warn-sub {
  font-size: 11.5px;
  font-weight: 700;
  color: var(--color-danger);
  letter-spacing: 0.01em;
}

/* ── Footer ── */
.reset-footer {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 24px 16px;
  border-top: 1px solid var(--color-surface);
  flex-shrink: 0;
}
.understood-text {
  font-size: 11px;
  font-weight: 500;
  color: var(--color-text-muted);
  flex-shrink: 0;
}
.footer-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-left: auto;
}

.cancel-btn {
  padding: 6px 16px;
  border-radius: 8px;
  font-size: 11.5px;
  font-weight: 600;
  cursor: pointer;
  border: none;
  background: var(--color-accent-bg);
  color: var(--color-accent-text);
  transition: 0.15s;
}
.cancel-btn:hover {
  background: var(--color-accent-border);
  color: var(--color-text);
}

.confirm-btn {
  padding: 6px 16px;
  border-radius: 8px;
  font-size: 11.5px;
  font-weight: 600;
  cursor: default;
  border: none;
  background: var(--color-surface);
  color: var(--color-text-muted);
  opacity: 0.55;
  transition: 0.15s;
  font-variant-numeric: tabular-nums;
}
.confirm-btn.ready {
  opacity: 1;
  cursor: pointer;
  background: var(--color-danger-bg);
  color: var(--color-danger);
}
.confirm-btn.ready:hover {
  background: var(--color-danger);
  color: #fff;
}
</style>
