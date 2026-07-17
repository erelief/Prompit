<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useSettingsWindow } from "../composables/useSettingsWindow";
import { appConfig, flushConfigSave } from "../stores/config";
import { ArrowLeft, Eye, EyeOff, Save, PlugZap } from "@lucide/vue";

const { t } = useI18n();
const router = useRouter();
const { growAbove } = useSettingsWindow();

type Status = { kind: "idle" | "info" | "success" | "error"; msg: string };
const status = ref<Status>({ kind: "idle", msg: "" });

// Form state mirrors appConfig.webdav; the account password is never part of
// the config — it goes to the OS credential store on save.
const url = ref(appConfig.webdav.url);
const username = ref(appConfig.webdav.username);
const remoteDir = ref(appConfig.webdav.remote_dir || "prompit");
const fileName = ref(appConfig.webdav.file_name || "prompit-backup.json");
const serverPw = ref("");
const showServerPw = ref(false);
const serverPwSaved = ref(false);
const testing = ref(false);
const saving = ref(false);

/** DTO matching `WebdavConnection` in commands/webdav.rs. An empty password
 * field means "use the keyring-stored one" (None on the Rust side). */
function connDto() {
  return {
    url: url.value.trim(),
    username: username.value.trim(),
    password: serverPw.value || null,
    remoteDir: remoteDir.value.trim() || "prompit",
  };
}

async function refreshPwStatus() {
  try {
    serverPwSaved.value = await invoke<boolean>("webdav_has_password");
  } catch {
    // Keyring probe failing is non-fatal; the operation itself will surface it.
  }
}

async function saveServer() {
  saving.value = true;
  status.value = { kind: "idle", msg: "" };
  try {
    appConfig.webdav.url = url.value.trim();
    appConfig.webdav.username = username.value.trim();
    appConfig.webdav.remote_dir = remoteDir.value.trim() || "prompit";
    appConfig.webdav.file_name = fileName.value.trim() || "prompit-backup.json";
    await flushConfigSave();
    if (serverPw.value) {
      await invoke("webdav_save_password", { password: serverPw.value });
      serverPw.value = "";
    }
    await refreshPwStatus();
    status.value = { kind: "success", msg: t("settings.webdav.server.saved") };
  } catch (err) {
    status.value = {
      kind: "error",
      msg: t("settings.webdav.error", { message: String(err) }),
    };
  } finally {
    saving.value = false;
  }
}

async function testConn() {
  testing.value = true;
  status.value = { kind: "idle", msg: "" };
  try {
    const r = await invoke<{ dirExists: boolean }>("webdav_test_connection", { conn: connDto() });
    status.value = {
      kind: "success",
      msg: r.dirExists
        ? t("settings.webdav.server.testSuccess")
        : t("settings.webdav.server.testSuccessNewDir"),
    };
  } catch (err) {
    status.value = {
      kind: "error",
      msg: t("settings.webdav.error", { message: String(err) }),
    };
  } finally {
    testing.value = false;
  }
}

onMounted(() => {
  void refreshPwStatus();
});

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
      <span class="header-title">{{ t('settings.webdav.pageTitle') }}</span>
    </div>

    <!-- Body -->
    <div class="ud-body">
      <p class="ud-desc">{{ t('settings.webdav.server.description') }}</p>

      <div class="field-label">{{ t('settings.webdav.server.urlLabel') }}</div>
      <div class="pw-row">
        <input
          class="pw-input"
          v-model="url"
          :placeholder="t('settings.webdav.server.urlPlaceholder')"
          autocomplete="off"
          spellcheck="false"
        />
      </div>

      <div class="field-label">{{ t('settings.webdav.server.usernameLabel') }}</div>
      <div class="pw-row">
        <input
          class="pw-input"
          v-model="username"
          :placeholder="t('settings.webdav.server.usernamePlaceholder')"
          autocomplete="off"
        />
      </div>

      <div class="field-label">{{ t('settings.webdav.server.passwordLabel') }}</div>
      <div class="pw-row">
        <input
          :type="showServerPw ? 'text' : 'password'"
          class="pw-input"
          v-model="serverPw"
          :placeholder="t('settings.webdav.server.passwordPlaceholder')"
          autocomplete="new-password"
        />
        <span v-if="serverPwSaved && !serverPw" class="saved-badge">
          {{ t('settings.webdav.server.passwordSaved') }}
        </span>
        <button class="pw-toggle" @click="showServerPw = !showServerPw" type="button">
          <Eye v-if="!showServerPw" :size="13" />
          <EyeOff v-else :size="13" />
        </button>
      </div>

      <div class="field-label">{{ t('settings.webdav.server.remoteDirLabel') }}</div>
      <div class="pw-row">
        <input
          class="pw-input"
          v-model="remoteDir"
          :placeholder="t('settings.webdav.server.remoteDirPlaceholder')"
          autocomplete="off"
          spellcheck="false"
        />
      </div>

      <div class="field-label">{{ t('settings.webdav.server.fileNameLabel') }}</div>
      <div class="pw-row">
        <input
          class="pw-input"
          v-model="fileName"
          :placeholder="t('settings.webdav.server.fileNamePlaceholder')"
          autocomplete="off"
          spellcheck="false"
        />
      </div>
      <p class="ud-hint">{{ t('settings.webdav.server.fileNameHint') }}</p>

      <div class="wd-actions">
        <button
          class="ud-btn analyze-btn"
          :disabled="!url.trim() || testing"
          @click="testConn"
        >
          <PlugZap :size="12" :stroke-width="1.9" />{{
            testing ? t('settings.webdav.server.testing') : t('settings.webdav.server.test')
          }}
        </button>
        <button
          class="ud-btn primary-btn"
          :disabled="!url.trim() || saving"
          @click="saveServer"
        >
          <Save :size="12" :stroke-width="1.9" />{{ t('settings.webdav.server.save') }}
        </button>
      </div>

      <p
        v-if="status.kind !== 'idle'"
        class="status-text"
        :class="{
          success: status.kind === 'success',
          error: status.kind === 'error',
          info: status.kind === 'info',
        }"
      >{{ status.msg }}</p>
    </div>
  </div>
</template>

<style scoped>
.ud-body { order: 0; }

.ud-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px 24px 12px;
  border-bottom: 1px solid var(--color-surface);
  flex-shrink: 0;
}

.ud-body {
  flex: 1;
  overflow-y: auto;
  padding: 18px 24px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.ud-body::-webkit-scrollbar { width: 3px; }
.ud-body::-webkit-scrollbar-thumb { background: var(--color-scrollbar); border-radius: 3px; }

.ud-hint {
  font-size: 10px;
  font-weight: 500;
  color: var(--color-text-muted);
  margin-top: -4px;
}
.field-label {
  font-size: 10.5px;
  font-weight: 650;
  color: var(--color-text-secondary);
  letter-spacing: 0.01em;
  margin-top: 2px;
}

.wd-actions {
  display: flex;
  gap: 8px;
  margin-top: 4px;
}
.wd-actions .ud-btn {
  flex: 1;
}
.saved-badge {
  font-size: 9px;
  font-weight: 650;
  letter-spacing: 0.03em;
  text-transform: uppercase;
  color: var(--color-success);
  flex-shrink: 0;
}

.analyze-btn {
  color: var(--color-accent-text);
  background: var(--color-accent-bg);
  border-color: var(--color-accent-border);
}
.analyze-btn:hover:not(:disabled) {
  background: var(--color-accent);
  color: var(--color-bg);
}

</style>
