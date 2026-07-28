<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useSettingsWindow } from "../composables/useSettingsWindow";
import { appConfig, flushConfigSave } from "../stores/config";
import { ArrowLeft, Eye, EyeOff, Save, PlugZap, KeyRound } from "@lucide/vue";

const { t } = useI18n();
const router = useRouter();
const { growAbove } = useSettingsWindow();

type Status = { kind: "idle" | "info" | "success" | "error"; msg: string };
const status = ref<Status>({ kind: "idle", msg: "" });

// Form state mirrors appConfig.webdav (URL/username/remote dir); the account
// password is never part of the config — it goes to the OS credential store on
// save. The backup file name is chosen on the Backup page, not here.
const url = ref(appConfig.webdav.url);
const username = ref(appConfig.webdav.username);
const remoteDir = ref(appConfig.webdav.remote_dir || "prompit");
const serverPw = ref("");
const showServerPw = ref(false);
const serverPwSaved = ref(false);
// When a password is already stored, the field is hidden behind a "Change"
// button until the user opts in — avoids an always-empty password box on the
// edit page. `editingPw` flips to true either by clicking that button or when
// no password is stored yet.
const editingPw = ref(false);
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
    // No stored password yet → the field must be editable so the user can set
    // one. A stored password stays hidden behind the "Change" button.
    if (!serverPwSaved.value) editingPw.value = true;
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
    await flushConfigSave();
    if (serverPw.value) {
      await invoke("webdav_save_password", { password: serverPw.value });
      serverPw.value = "";
    }
    await refreshPwStatus();
    // After a successful save the password lives in the keyring; collapse the
    // field back to the "Change" button unless nothing got stored.
    if (serverPwSaved.value) editingPw.value = false;
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
      <div v-if="serverPwSaved && !editingPw" class="pw-row pw-saved-row">
        <KeyRound :size="13" class="pw-saved-icon" />
        <span class="pw-saved-text">{{ t('settings.webdav.server.passwordSavedLabel') }}</span>
        <button
          class="pw-change-btn"
          type="button"
          @click="editingPw = true"
        >{{ t('settings.webdav.server.passwordChange') }}</button>
      </div>
      <div v-else class="pw-row">
        <input
          :type="showServerPw ? 'text' : 'password'"
          class="pw-input"
          v-model="serverPw"
          :placeholder="t('settings.webdav.server.passwordPlaceholder')"
          autocomplete="new-password"
        />
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

.ud-body {
  flex: 1;
  overflow-y: auto;
  padding: 18px var(--space-6);
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.ud-body::-webkit-scrollbar { width: 3px; }
.ud-body::-webkit-scrollbar-thumb { background: var(--color-scrollbar); border-radius: var(--radius-xs); }

.ud-hint {
  font-size: var(--text-xs);
  font-weight: var(--weight-medium);
  color: var(--color-text-muted);
  margin-top: -4px;
}

.wd-actions {
  display: flex;
  gap: var(--space-2);
  margin-top: var(--space-1);
}
.wd-actions .ud-btn {
  flex: 1;
}
/* The saved-password row reuses the surrounding .pw-row frame; inside it the
   status text matches the input text size/weight, and "Change" is a compact
   accent button styled like .pw-toggle. */
.pw-saved-row { gap: var(--space-2); }
.pw-saved-icon { color: var(--color-text-muted); flex-shrink: 0; }
.pw-saved-text {
  flex: 1;
  font-size: var(--text-base);
  color: var(--color-text);
}
.pw-change-btn {
  flex-shrink: 0;
  padding: 3px 10px;
  border-radius: var(--radius-sm);
  border: none;
  background: none;
  color: var(--color-accent-text);
  font-size: var(--text-xs);
  font-family: inherit;
  cursor: pointer;
  transition: background .12s, color .12s;
}
.pw-change-btn:hover { background: var(--color-accent-bg); }
.pw-change-btn:focus-visible { outline: 2px solid var(--color-accent-border); outline-offset: 1px; }

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

</style>
