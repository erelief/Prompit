<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { ArrowLeft, ShieldAlert } from "@lucide/vue";

const { t } = useI18n();
const router = useRouter();

const countdown = ref(5);
const ready = ref(false);
const isSandbox = ref(false);
let timer: ReturnType<typeof setInterval> | null = null;

onMounted(async () => {
  isSandbox.value = await invoke<boolean>("is_sandbox");
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
      <span v-if="isSandbox" class="sandbox-badge">{{ t('settings.reset.sandboxBadge') }}</span>
    </div>

    <!-- Body -->
    <div class="reset-body">
      <div class="warn-card">
        <div class="warn-icon-wrap">
          <ShieldAlert :size="22" :stroke-width="1.5" />
        </div>
        <div class="warn-content">
          <p class="warn-text">{{ t('settings.reset.warning') }}</p>
          <p class="warn-irreversible">{{ t('settings.reset.irreversible') }}</p>
        </div>
      </div>
      <p v-if="isSandbox" class="sandbox-hint">{{ t('settings.reset.sandboxHint') }}</p>
    </div>

    <!-- Footer -->
    <div class="reset-footer">
      <span class="understood-label">{{ t('settings.reset.understood') }}</span>
      <div class="footer-actions">
        <button class="action-btn cancel-btn" @click="cancel">
          {{ t('common.cancel') }}
        </button>
        <button
          class="action-btn confirm-btn"
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

/* ── Body ── */
.reset-body {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px 28px;
}
.warn-card {
  display: flex;
  align-items: flex-start;
  gap: 16px;
  max-width: 340px;
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
  margin-top: 12px;
  font-size: 11px;
  font-weight: 500;
  color: #16a34a;
  text-align: center;
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
.understood-label {
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

/* ── Action buttons (shared base) ── */
.action-btn {
  padding: 6px 16px;
  border-radius: 8px;
  font-size: 11.5px;
  font-weight: 600;
  cursor: pointer;
  border: none;
  transition: all 0.15s;
}

/* Cancel: always prominent (accent) */
.cancel-btn {
  background: var(--color-accent-bg);
  color: var(--color-accent-text);
}
.cancel-btn:hover {
  background: var(--color-accent-border);
  color: var(--color-text);
}

/* Confirm: countdown -> red */
.confirm-btn {
  cursor: default;
  background: var(--color-surface);
  color: var(--color-text-muted);
  opacity: 0.55;
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
