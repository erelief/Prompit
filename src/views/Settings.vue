<script setup lang="ts">
import { ref, onMounted } from "vue";

interface ModelConfig {
  api_key: string;
  base_url: string;
  model: string;
  display_name: string;
  temperature: number | null;
  max_tokens: number | null;
}

interface AppConfig {
  models: ModelConfig[];
  selected_model_index: number;
  target_lang: string;
  privacy_mode: boolean;
  translation_mode: string;
  persona: string;
}

const config = ref<AppConfig>({
  models: [],
  selected_model_index: 0,
  target_lang: "English",
  privacy_mode: false,
  translation_mode: "manual",
  persona: "",
});

const statusMessage = ref("");

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

async function loadConfig() {
  // Will be wired to Tauri invoke in Task 9
  console.log("load config");
}

async function saveConfig() {
  statusMessage.value = "";
  try {
    // Will be wired to Tauri invoke in Task 9
    console.log("save config", config.value);
    statusMessage.value = "Saved!";
    setTimeout(() => (statusMessage.value = ""), 2000);
  } catch (err) {
    statusMessage.value = `Error: ${err}`;
  }
}

function addModel() {
  config.value.models.push({
    api_key: "",
    base_url: "https://api.openai.com/v1",
    model: "gpt-4o-mini",
    display_name: "",
    temperature: 0.3,
    max_tokens: 1024,
  });
}

function removeModel(index: number) {
  config.value.models.splice(index, 1);
  if (
    config.value.selected_model_index >= config.value.models.length
  ) {
    config.value.selected_model_index = Math.max(
      0,
      config.value.models.length - 1
    );
  }
}

onMounted(() => {
  loadConfig();
});
</script>

<template>
  <div class="min-h-screen bg-gray-900 text-white p-6">
    <div class="max-w-2xl mx-auto">
      <h1 class="text-xl font-bold mb-6">Settings</h1>

      <!-- Target Language -->
      <section class="mb-6">
        <h2 class="text-sm font-semibold text-white/70 mb-2">
          Target Language
        </h2>
        <select
          v-model="config.target_lang"
          class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2
                 text-sm focus:border-blue-500 outline-none"
        >
          <option v-for="lang in targetLanguages" :key="lang" :value="lang">
            {{ lang }}
          </option>
        </select>
      </section>

      <!-- Translation Mode -->
      <section class="mb-6">
        <h2 class="text-sm font-semibold text-white/70 mb-2">
          Translation Mode
        </h2>
        <div class="flex gap-4">
          <label class="flex items-center gap-2 text-sm cursor-pointer">
            <input
              type="radio"
              v-model="config.translation_mode"
              value="manual"
              class="accent-blue-500"
            />
            Manual (Enter to translate)
          </label>
          <label class="flex items-center gap-2 text-sm cursor-pointer">
            <input
              type="radio"
              v-model="config.translation_mode"
              value="realtime"
              class="accent-blue-500"
            />
            Realtime (auto after debounce)
          </label>
        </div>
      </section>

      <!-- Privacy Mode -->
      <section class="mb-6">
        <label class="flex items-center gap-3 text-sm cursor-pointer">
          <input
            type="checkbox"
            v-model="config.privacy_mode"
            class="accent-blue-500 w-4 h-4"
          />
          Privacy Mode (use local AIFW service)
        </label>
      </section>

      <!-- Models -->
      <section class="mb-6">
        <div class="flex items-center justify-between mb-2">
          <h2 class="text-sm font-semibold text-white/70">Models</h2>
          <button
            @click="addModel"
            class="text-xs bg-blue-600 hover:bg-blue-500 px-3 py-1
                   rounded transition-colors"
          >
            + Add Model
          </button>
        </div>

        <div
          v-for="(model, index) in config.models"
          :key="index"
          class="bg-gray-800 border border-gray-700 rounded-lg p-4 mb-3"
        >
          <div class="flex items-center justify-between mb-3">
            <label class="flex items-center gap-2 text-sm cursor-pointer">
              <input
                type="radio"
                v-model="config.selected_model_index"
                :value="index"
                class="accent-blue-500"
              />
              <span class="text-white/60">Active</span>
            </label>
            <button
              @click="removeModel(index)"
              class="text-xs text-red-400 hover:text-red-300 transition-colors"
            >
              Remove
            </button>
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="text-xs text-white/50 mb-1 block"
                >Display Name</label
              >
              <input
                v-model="model.display_name"
                class="w-full bg-gray-700 border border-gray-600 rounded px-2
                       py-1.5 text-sm focus:border-blue-500 outline-none"
              />
            </div>
            <div>
              <label class="text-xs text-white/50 mb-1 block">Model</label>
              <input
                v-model="model.model"
                class="w-full bg-gray-700 border border-gray-600 rounded px-2
                       py-1.5 text-sm focus:border-blue-500 outline-none"
              />
            </div>
            <div class="col-span-2">
              <label class="text-xs text-white/50 mb-1 block">Base URL</label>
              <input
                v-model="model.base_url"
                class="w-full bg-gray-700 border border-gray-600 rounded px-2
                       py-1.5 text-sm focus:border-blue-500 outline-none"
              />
            </div>
            <div class="col-span-2">
              <label class="text-xs text-white/50 mb-1 block">API Key</label>
              <input
                v-model="model.api_key"
                type="password"
                class="w-full bg-gray-700 border border-gray-600 rounded px-2
                       py-1.5 text-sm focus:border-blue-500 outline-none"
              />
            </div>
          </div>
        </div>

        <div
          v-if="config.models.length === 0"
          class="text-white/40 text-sm text-center py-4"
        >
          No models configured. Click "+ Add Model" to add one.
        </div>
      </section>

      <!-- Persona -->
      <section class="mb-6">
        <h2 class="text-sm font-semibold text-white/70 mb-2">
          Translation Persona (optional)
        </h2>
        <input
          v-model="config.persona"
          placeholder="e.g. formal, casual, technical..."
          class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2
                 text-sm focus:border-blue-500 outline-none"
        />
      </section>

      <!-- Save -->
      <div class="flex items-center gap-3">
        <button
          @click="saveConfig"
          class="bg-blue-600 hover:bg-blue-500 px-6 py-2 rounded-lg
                 text-sm font-medium transition-colors"
        >
          Save
        </button>
        <span
          v-if="statusMessage"
          :class="
            statusMessage.startsWith('Error')
              ? 'text-red-400'
              : 'text-green-400'
          "
          class="text-sm"
        >
          {{ statusMessage }}
        </span>
      </div>
    </div>
  </div>
</template>
