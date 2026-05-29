<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useRouter } from "vue-router";
import {
  appConfig,
  loadConfig,
  saveConfig as persistConfig,
} from "../stores/config";
import {
  ArrowLeft,
  Languages,
  Cpu,
  UserCircle,
  Plus,
  Trash2,
  CircleCheck,
  Circle,
  Check,
  Eye,
  EyeOff,
} from "@lucide/vue";

const router = useRouter();
const statusMessage = ref("");
const visibleKeys = ref<Set<number>>(new Set());

function toggleKeyVisibility(index: number) {
  const s = new Set(visibleKeys.value);
  if (s.has(index)) {
    s.delete(index);
  } else {
    s.add(index);
  }
  visibleKeys.value = s;
}

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
  const s = new Set(visibleKeys.value);
  s.delete(index);
  const reindexed = new Set<number>();
  for (const i of s) {
    reindexed.add(i > index ? i - 1 : i);
  }
  visibleKeys.value = reindexed;
  if (appConfig.selected_model_index >= appConfig.models.length) {
    appConfig.selected_model_index = Math.max(0, appConfig.models.length - 1);
  }
}

async function goBack() {
  await invoke("resize_main_window", { width: 600, height: 200 });
  router.push("/");
}

async function handleDrag(e: MouseEvent) {
  const target = e.target as HTMLElement;
  if (target.closest("textarea, button, input, select, a, .model-card")) return;
  await getCurrentWindow().startDragging();
}

onMounted(async () => {
  await invoke("resize_main_window", { width: 660, height: 420 });
  load();
});
</script>

<template>
  <div class="settings-root" @mousedown="handleDrag">
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
                <div class="key-input-wrapper">
                  <input
                    v-model="model.api_key"
                    :type="visibleKeys.has(index) ? 'text' : 'password'"
                    class="field-input key-input"
                  />
                  <button
                    type="button"
                    class="eye-btn"
                    @click.stop="toggleKeyVisibility(index)"
                    :title="visibleKeys.has(index) ? 'Hide key' : 'Show key'"
                  >
                    <EyeOff v-if="visibleKeys.has(index)" :size="14" :stroke-width="1.8" />
                    <Eye v-else :size="14" :stroke-width="1.8" />
                  </button>
                </div>
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

/* Radio dot (used in model cards) */
.radio-dot {
  color: rgba(255, 255, 255, 0.2);
  display: flex;
  flex-shrink: 0;
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

.key-input-wrapper {
  position: relative;
}

.key-input {
  padding-right: 32px !important;
}

.eye-btn {
  position: absolute;
  right: 6px;
  top: 50%;
  transform: translateY(-50%);
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: 4px;
  color: rgba(255, 255, 255, 0.2);
  transition: all 0.15s ease;
}

.eye-btn:hover {
  color: rgba(255, 255, 255, 0.7);
  background: rgba(255, 255, 255, 0.06);
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
