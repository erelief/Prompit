<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useRouter } from "vue-router";
import {
  appConfig,
  loadConfig,
  saveConfig as persistConfig,
} from "../stores/config";
import {
  ArrowLeft,
  Languages,
  Keyboard,
  Shield,
  Cpu,
  UserCircle,
  Server,
  Plus,
  Trash2,
  CircleCheck,
  Circle,
  Check,
} from "@lucide/vue";

const router = useRouter();
const statusMessage = ref("");
const aifwExePath = ref("aifw_server.exe");
const aifwRunning = ref(false);

const targetLanguages = [
  "English",
  "Chinese",
  "Japanese",
  "Korean",
  "French",
  "German",
  "Spanish",
  "Russian",
];

async function load() {
  try {
    await loadConfig();
  } catch (err) {
    console.error("Failed to load config:", err);
  }
}

async function saveConfig() {
  statusMessage.value = "";
  try {
    await persistConfig();
    statusMessage.value = "Saved!";
    setTimeout(() => (statusMessage.value = ""), 2500);
  } catch (err) {
    statusMessage.value = `Error: ${err}`;
  }
}

function addModel() {
  appConfig.models.push({
    api_key: "",
    base_url: "https://api.openai.com/v1",
    model: "gpt-4o-mini",
    display_name: "",
    temperature: 0.3,
    max_tokens: 1024,
  });
}

function removeModel(index: number) {
  appConfig.models.splice(index, 1);
  if (appConfig.selected_model_index >= appConfig.models.length) {
    appConfig.selected_model_index = Math.max(0, appConfig.models.length - 1);
  }
}

async function checkAifwStatus() {
  try {
    aifwRunning.value = await invoke<boolean>("aifw_status");
  } catch {
    aifwRunning.value = false;
  }
}

async function startAifw() {
  try {
    await invoke("start_aifw", { exePath: aifwExePath.value });
    aifwRunning.value = true;
  } catch (err) {
    statusMessage.value = `AIFW Error: ${err}`;
  }
}

async function stopAifw() {
  try {
    await invoke("stop_aifw");
    aifwRunning.value = false;
  } catch (err) {
    statusMessage.value = `AIFW Error: ${err}`;
  }
}

async function goBack() {
  await invoke("resize_main_window", { width: 600, height: 200 });
  router.push("/");
}

onMounted(async () => {
  await invoke("resize_main_window", { width: 660, height: 580 });
  load();
  checkAifwStatus();
});
</script>

<template>
  <div class="settings-root">
    <!-- Header -->
    <header class="settings-header">
      <button @click="goBack" class="back-btn" title="Back">
        <ArrowLeft :size="18" :stroke-width="1.8" />
      </button>
      <div>
        <h1 class="text-[15px] font-semibold tracking-tight">Settings</h1>
        <p class="text-[11px] text-white/30">Configure your translator</p>
      </div>
    </header>

    <div class="settings-body">
      <!-- Target Language -->
      <section class="settings-section">
        <div class="section-label">
          <Languages :size="14" :stroke-width="1.8" />
          <span>Target Language</span>
        </div>
        <select v-model="appConfig.target_lang" class="settings-select">
          <option v-for="lang in targetLanguages" :key="lang" :value="lang">
            {{ lang }}
          </option>
        </select>
      </section>

      <!-- Translation Mode -->
      <section class="settings-section">
        <div class="section-label">
          <Keyboard :size="14" :stroke-width="1.8" />
          <span>Translation Mode</span>
        </div>
        <div class="radio-group">
          <label
            class="radio-option"
            :class="{ active: appConfig.translation_mode === 'manual' }"
          >
            <input
              type="radio"
              v-model="appConfig.translation_mode"
              value="manual"
              class="sr-only"
            />
            <span class="radio-dot">
              <CircleCheck v-if="appConfig.translation_mode === 'manual'" :size="16" />
              <Circle v-else :size="16" />
            </span>
            <div>
              <div class="text-[12px] text-white/80">Manual</div>
              <div class="text-[10px] text-white/30">Press Enter to translate</div>
            </div>
          </label>
          <label
            class="radio-option"
            :class="{ active: appConfig.translation_mode === 'realtime' }"
          >
            <input
              type="radio"
              v-model="appConfig.translation_mode"
              value="realtime"
              class="sr-only"
            />
            <span class="radio-dot">
              <CircleCheck v-if="appConfig.translation_mode === 'realtime'" :size="16" />
              <Circle v-else :size="16" />
            </span>
            <div>
              <div class="text-[12px] text-white/80">Realtime</div>
              <div class="text-[10px] text-white/30">Auto after debounce</div>
            </div>
          </label>
        </div>
      </section>

      <!-- Privacy Mode -->
      <section class="settings-section">
        <div class="section-label">
          <Shield :size="14" :stroke-width="1.8" />
          <span>Privacy Mode</span>
        </div>
        <label class="toggle-row" @click.prevent="appConfig.privacy_mode = !appConfig.privacy_mode">
          <span class="text-[12px] text-white/60">Use local AIFW service for private translation</span>
          <div class="toggle-switch" :class="{ on: appConfig.privacy_mode }">
            <div class="toggle-thumb"></div>
          </div>
        </label>
      </section>

      <!-- Models -->
      <section class="settings-section">
        <div class="section-label">
          <Cpu :size="14" :stroke-width="1.8" />
          <span>Models</span>
          <button @click="addModel" class="add-btn ml-auto">
            <Plus :size="13" :stroke-width="2" />
            <span>Add</span>
          </button>
        </div>

        <div class="model-list">
          <div
            v-for="(model, index) in appConfig.models"
            :key="index"
            class="model-card"
            :class="{ selected: appConfig.selected_model_index === index }"
            @click="appConfig.selected_model_index = index"
          >
            <div class="model-card-header">
              <span class="radio-dot" @click.stop>
                <CircleCheck
                  v-if="appConfig.selected_model_index === index"
                  :size="15"
                />
                <Circle v-else :size="15" />
              </span>
              <span class="text-[11px] text-white/40 flex-1">
                {{ model.display_name || model.model || "Unnamed Model" }}
              </span>
              <button
                @click.stop="removeModel(index)"
                class="delete-btn"
                title="Remove"
              >
                <Trash2 :size="13" :stroke-width="1.8" />
              </button>
            </div>

            <div class="model-grid">
              <div class="field">
                <label>Display Name</label>
                <input v-model="model.display_name" class="field-input" />
              </div>
              <div class="field">
                <label>Model</label>
                <input v-model="model.model" class="field-input" />
              </div>
              <div class="field col-span-2">
                <label>Base URL</label>
                <input v-model="model.base_url" class="field-input" />
              </div>
              <div class="field col-span-2">
                <label>API Key</label>
                <input
                  v-model="model.api_key"
                  type="password"
                  class="field-input"
                />
              </div>
            </div>
          </div>

          <div
            v-if="appConfig.models.length === 0"
            class="empty-state"
          >
            <Cpu :size="20" :stroke-width="1.2" class="text-white/15" />
            <span>No models configured</span>
          </div>
        </div>
      </section>

      <!-- Persona -->
      <section class="settings-section">
        <div class="section-label">
          <UserCircle :size="14" :stroke-width="1.8" />
          <span>Translation Persona</span>
          <span class="text-[10px] text-white/20 ml-auto font-normal">Optional</span>
        </div>
        <input
          v-model="appConfig.persona"
          placeholder="e.g. formal, casual, technical..."
          class="settings-input"
        />
      </section>

      <!-- AIFW Control -->
      <section class="settings-section">
        <div class="section-label">
          <Server :size="14" :stroke-width="1.8" />
          <span>AIFW Service</span>
          <span
            class="status-badge ml-auto"
            :class="aifwRunning ? 'status-on' : 'status-off'"
          >
            <span class="status-dot"></span>
            {{ aifwRunning ? "Running" : "Stopped" }}
          </span>
        </div>
        <div class="aifw-row">
          <input
            v-model="aifwExePath"
            placeholder="Path to aifw_server.exe"
            class="settings-input flex-1"
          />
          <button
            v-if="!aifwRunning"
            @click="startAifw"
            class="action-btn action-green"
          >
            <span class="inline-block w-1.5 h-1.5 rounded-full bg-current"></span>
            Start
          </button>
          <button v-else @click="stopAifw" class="action-btn action-red">
            <span class="inline-block w-1.5 h-1.5 rounded-full bg-current"></span>
            Stop
          </button>
        </div>
      </section>
    </div>

    <!-- Save footer -->
    <footer class="settings-footer">
      <button @click="saveConfig" class="save-btn" :class="{ success: statusMessage === 'Saved!' }">
        <Check v-if="statusMessage === 'Saved!'" :size="15" :stroke-width="2.5" />
        <span v-else>Save Changes</span>
      </button>
      <Transition name="fade">
        <span
          v-if="statusMessage && statusMessage !== 'Saved!'"
          class="text-[11px] text-red-400/80"
        >
          {{ statusMessage }}
        </span>
      </Transition>
    </footer>
  </div>
</template>

<style scoped>
.settings-root {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: #0c0c10;
  color: #fff;
  overflow: hidden;
}

/* Header */
.settings-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px 24px 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.04);
  flex-shrink: 0;
}

.back-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 8px;
  color: rgba(255, 255, 255, 0.4);
  transition: all 0.15s ease;
}

.back-btn:hover {
  color: rgba(255, 255, 255, 0.8);
  background: rgba(255, 255, 255, 0.06);
}

/* Body */
.settings-body {
  flex: 1;
  overflow-y: auto;
  padding: 8px 24px 20px;
}

.settings-body::-webkit-scrollbar {
  width: 4px;
}

.settings-body::-webkit-scrollbar-track {
  background: transparent;
}

.settings-body::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.08);
  border-radius: 4px;
}

/* Sections */
.settings-section {
  padding: 14px 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.04);
}

.settings-section:last-child {
  border-bottom: none;
}

.section-label {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.55);
  margin-bottom: 10px;
  letter-spacing: 0.01em;
}

/* Inputs */
.settings-input {
  width: 100%;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  padding: 8px 12px;
  font-size: 12px;
  color: rgba(255, 255, 255, 0.85);
  outline: none;
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
}

.settings-input::placeholder {
  color: rgba(255, 255, 255, 0.2);
}

.settings-input:focus {
  border-color: rgba(217, 160, 71, 0.3);
  box-shadow: 0 0 0 2px rgba(217, 160, 71, 0.06);
}

.settings-select {
  width: 100%;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  padding: 8px 12px;
  font-size: 12px;
  color: rgba(255, 255, 255, 0.85);
  outline: none;
  cursor: pointer;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='rgba(255,255,255,0.3)' stroke-width='2'%3E%3Cpolyline points='6 9 12 15 18 9'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 12px center;
  transition: border-color 0.15s ease;
}

.settings-select:focus {
  border-color: rgba(217, 160, 71, 0.3);
}

.settings-select option {
  background: #1a1a22;
  color: #fff;
}

/* Radio group */
.radio-group {
  display: flex;
  gap: 8px;
}

.radio-option {
  display: flex;
  align-items: center;
  gap: 10px;
  flex: 1;
  padding: 10px 12px;
  border-radius: 9px;
  border: 1px solid rgba(255, 255, 255, 0.06);
  background: rgba(255, 255, 255, 0.02);
  cursor: pointer;
  transition: all 0.15s ease;
}

.radio-option:hover {
  border-color: rgba(255, 255, 255, 0.1);
  background: rgba(255, 255, 255, 0.04);
}

.radio-option.active {
  border-color: rgba(217, 160, 71, 0.25);
  background: rgba(217, 160, 71, 0.05);
}

.radio-dot {
  color: rgba(255, 255, 255, 0.2);
  display: flex;
  flex-shrink: 0;
}

.radio-option.active .radio-dot {
  color: #d4a048;
}

/* Toggle */
.toggle-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  border-radius: 9px;
  border: 1px solid rgba(255, 255, 255, 0.06);
  background: rgba(255, 255, 255, 0.02);
  cursor: pointer;
  transition: border-color 0.15s ease;
}

.toggle-row:hover {
  border-color: rgba(255, 255, 255, 0.1);
}

.toggle-switch {
  width: 36px;
  height: 20px;
  border-radius: 20px;
  background: rgba(255, 255, 255, 0.1);
  position: relative;
  transition: background 0.2s ease;
  flex-shrink: 0;
}

.toggle-switch.on {
  background: rgba(217, 160, 71, 0.5);
}

.toggle-thumb {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.7);
  transition: all 0.2s ease;
}

.toggle-switch.on .toggle-thumb {
  left: 19px;
  background: #d4a048;
}

/* Model list */
.model-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.model-card {
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.02);
  padding: 12px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.model-card:hover {
  border-color: rgba(255, 255, 255, 0.1);
  background: rgba(255, 255, 255, 0.03);
}

.model-card.selected {
  border-color: rgba(217, 160, 71, 0.2);
  background: rgba(217, 160, 71, 0.03);
}

.model-card-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 10px;
}

.model-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}

.col-span-2 {
  grid-column: span 2;
}

.field label {
  display: block;
  font-size: 10px;
  color: rgba(255, 255, 255, 0.3);
  margin-bottom: 4px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.field-input {
  width: 100%;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 6px;
  padding: 6px 10px;
  font-size: 12px;
  color: rgba(255, 255, 255, 0.8);
  outline: none;
  transition: border-color 0.15s ease;
}

.field-input::placeholder {
  color: rgba(255, 255, 255, 0.15);
}

.field-input:focus {
  border-color: rgba(217, 160, 71, 0.3);
}

.delete-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border-radius: 6px;
  color: rgba(255, 255, 255, 0.2);
  transition: all 0.15s ease;
}

.delete-btn:hover {
  color: #f87171;
  background: rgba(248, 113, 113, 0.1);
}

.add-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  font-weight: 500;
  color: rgba(217, 160, 71, 0.7);
  padding: 4px 10px;
  border-radius: 6px;
  transition: all 0.15s ease;
}

.add-btn:hover {
  color: #d4a048;
  background: rgba(217, 160, 71, 0.1);
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 20px;
  font-size: 11px;
  color: rgba(255, 255, 255, 0.2);
}

/* Status badge */
.status-badge {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 10px;
  font-weight: 500;
  padding: 3px 8px 3px 6px;
  border-radius: 20px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.status-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
}

.status-on {
  color: #4ade80;
  background: rgba(74, 222, 128, 0.1);
}

.status-on .status-dot {
  background: #4ade80;
  box-shadow: 0 0 6px rgba(74, 222, 128, 0.4);
}

.status-off {
  color: rgba(255, 255, 255, 0.3);
  background: rgba(255, 255, 255, 0.04);
}

.status-off .status-dot {
  background: rgba(255, 255, 255, 0.25);
}

/* AIFW row */
.aifw-row {
  display: flex;
  gap: 8px;
  align-items: center;
}

.action-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 500;
  white-space: nowrap;
  transition: all 0.15s ease;
  flex-shrink: 0;
}

.action-green {
  background: rgba(74, 222, 128, 0.12);
  color: #4ade80;
  border: 1px solid rgba(74, 222, 128, 0.15);
}

.action-green:hover {
  background: rgba(74, 222, 128, 0.18);
  border-color: rgba(74, 222, 128, 0.25);
}

.action-red {
  background: rgba(248, 113, 113, 0.1);
  color: #f87171;
  border: 1px solid rgba(248, 113, 113, 0.12);
}

.action-red:hover {
  background: rgba(248, 113, 113, 0.16);
  border-color: rgba(248, 113, 113, 0.2);
}

/* Footer */
.settings-footer {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 24px 16px;
  border-top: 1px solid rgba(255, 255, 255, 0.04);
  flex-shrink: 0;
}

.save-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 110px;
  padding: 8px 20px;
  border-radius: 9px;
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.01em;
  background: linear-gradient(135deg, #d4a048 0%, #c4922e 100%);
  color: #1a1a1a;
  transition: all 0.15s ease;
}

.save-btn:hover {
  background: linear-gradient(135deg, #ddb35a 0%, #d4a048 100%);
  transform: translateY(-0.5px);
  box-shadow: 0 4px 12px rgba(212, 160, 72, 0.2);
}

.save-btn:active {
  transform: translateY(0);
}

.save-btn.success {
  background: rgba(74, 222, 128, 0.15);
  color: #4ade80;
  min-width: 40px;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
