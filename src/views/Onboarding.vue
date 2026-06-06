<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import {
  appConfig,
  loadProviderPresets,
} from "../stores/config";
import type { ProviderConfig, ProviderPreset } from "../stores/config";
import {
  testProviderConnection,
  fetchProviderModels,
} from "../services/llm-client";
import { BUILTIN_LANGUAGES, getLangName } from "../constants/languages";
import {
  ChevronRight,
  ChevronLeft,
  Loader2,
  Check,
  Eye,
  EyeOff,
  Zap,
  PartyPopper,
} from "@lucide/vue";

const { t } = useI18n();
const router = useRouter();

// ── Step management ──
const currentStep = ref(0);
const direction = ref<"forward" | "backward">("forward");

// ── Step 0: Language ──
const selectedLang = ref(appConfig.app_lang || "en");

// ── Step 2: Provider form ──
const providerForm = ref<ProviderConfig>({
  name: "",
  api_key: "",
  base_url: "",
  models: [],
  temperature: 0.3,
  max_tokens: 1024,
});
const providerPresets = ref<ProviderPreset[]>([]);
const selectedPreset = ref("");
const showApiKey = ref(false);

// ── Step 3: Models ──
const availableModels = ref<string[]>([]);
const selectedModels = ref(new Set<string>());
const fetchError = ref("");
const isConnecting = ref(false);
const isFetching = ref(false);

// ── Computed ──
const canProceed = computed(() => {
  switch (currentStep.value) {
    case 0: return true;
    case 1: return true;
    case 2:
      return (
        providerForm.value.name.trim() !== "" &&
        providerForm.value.api_key.trim() !== "" &&
        providerForm.value.base_url.trim() !== ""
      );
    case 3:
      return !isConnecting.value && !isFetching.value;
    case 4:
      return selectedModels.value.size > 0;
    default: return false;
  }
});

const isLastStep = computed(() => currentStep.value === 4);

const shortcutKey = computed(() => {
  const isMac = navigator.userAgent.includes("Mac");
  return isMac ? "⌥Y" : "Alt+Y";
});

// ── Navigation ──
function goNext() {
  if (!canProceed.value) return;
  if (currentStep.value === 2) {
    confirmProviderAndAdvance();
    return;
  }
  if (currentStep.value === 4) {
    finishOnboarding();
    return;
  }
  if (currentStep.value === 0) {
    applyLanguage();
  }
  direction.value = "forward";
  currentStep.value++;
}

function goPrev() {
  if (currentStep.value === 0) return;
  direction.value = "backward";
  currentStep.value--;
}

// ── Step 0 logic ──
function applyLanguage() {
  appConfig.app_lang = selectedLang.value;
}

// ── Step 2 logic ──
function applyPreset(presetName: string) {
  const preset = providerPresets.value.find((p) => p.name === presetName);
  if (!preset) return;
  selectedPreset.value = presetName;
  providerForm.value.name = preset.provider_name;
  providerForm.value.base_url = preset.base_url;
  providerForm.value.preset = presetName;
  providerForm.value.api_format = { ...preset.api_format };
}

// ── Step 3 flow ──
async function confirmProviderAndAdvance() {
  isConnecting.value = true;
  fetchError.value = "";

  const result = await testProviderConnection(providerForm.value);
  if (!result.ok) {
    fetchError.value = result.error || "Connection failed";
    isConnecting.value = false;
    return;
  }

  isConnecting.value = false;
  isFetching.value = true;

  const modelsResult = await fetchProviderModels(providerForm.value);
  if (!modelsResult.ok || !modelsResult.models || modelsResult.models.length === 0) {
    fetchError.value = modelsResult.error || "No models found";
    isFetching.value = false;
    return;
  }

  availableModels.value = modelsResult.models;
  isFetching.value = false;

  direction.value = "forward";
  currentStep.value = 3;
}

// ── Step 4 logic ──
function toggleModel(id: string) {
  const s = new Set(selectedModels.value);
  s.has(id) ? s.delete(id) : s.add(id);
  selectedModels.value = s;
}

function selectAll() {
  selectedModels.value = new Set(availableModels.value);
}

function deselectAll() {
  selectedModels.value = new Set();
}

async function finishOnboarding() {
  providerForm.value.models = [...selectedModels.value].map((id) => ({ id }));
  appConfig.providers.push({ ...providerForm.value });
  appConfig.active_provider_index = 0;
  appConfig.active_model_index = 0;

  await invoke("set_onboarding_complete");

  router.replace("/");
}

// ── Init ──
onMounted(async () => {
  try {
    providerPresets.value = await loadProviderPresets();
  } catch (err) {
    console.error("Failed to load presets:", err);
  }
});
</script>

<template>
  <div class="flex items-center justify-center min-h-dvh px-6" style="background: var(--color-bg)">
    <div class="w-full max-w-[480px] flex flex-col" style="min-height: 520px">

      <!-- Content area with transitions -->
      <div class="flex-1 relative overflow-hidden">
        <Transition :name="direction === 'forward' ? 'slide-left' : 'slide-right'" mode="out-in">

          <!-- Step 0: Welcome -->
          <div v-if="currentStep === 0" key="step0" class="flex flex-col items-center justify-center h-full py-10">
            <h1 class="text-5xl font-light tracking-tight mb-3" style="color: var(--color-text)">
              {{ t('onboarding.hello') }}
            </h1>
            <p class="text-base mb-10" style="color: var(--color-text-secondary)">
              {{ t('onboarding.welcomeTitle') }}
            </p>
            <div class="w-full max-w-xs">
              <label class="block text-xs font-medium mb-2 tracking-wide uppercase" style="color: var(--color-text-muted)">
                {{ t('onboarding.selectLanguage') }}
              </label>
              <select
                v-model="selectedLang"
                @change="applyLanguage()"
                class="w-full h-10 px-3 rounded-lg text-sm outline-none transition-colors cursor-pointer"
                style="background: var(--color-surface); color: var(--color-text); border: 1px solid var(--color-border)"
              >
                <option
                  v-for="lang in BUILTIN_LANGUAGES"
                  :key="lang"
                  :value="lang"
                >
                  {{ getLangName(lang) }}
                </option>
              </select>
            </div>
          </div>

          <!-- Step 1: Info -->
          <div v-else-if="currentStep === 1" key="step1" class="flex flex-col items-center justify-center h-full py-10">
            <div class="w-12 h-12 rounded-full flex items-center justify-center mb-6" style="background: var(--color-accent-bg)">
              <Zap :size="22" style="color: var(--color-accent)" />
            </div>
            <h2 class="text-xl font-medium mb-3" style="color: var(--color-text)">
              {{ t('onboarding.infoTitle') }}
            </h2>
            <p class="text-sm leading-relaxed text-center max-w-sm" style="color: var(--color-text-secondary)">
              {{ t('onboarding.infoBody') }}
            </p>
          </div>

          <!-- Step 2: Add Provider -->
          <div v-else-if="currentStep === 2" key="step2" class="flex flex-col py-6">
            <h2 class="text-lg font-medium mb-6" style="color: var(--color-text)">
              {{ t('onboarding.addProviderTitle') }}
            </h2>

            <!-- Preset selector -->
            <div v-if="providerPresets.length > 0" class="mb-5">
              <label class="block text-xs font-medium mb-1.5 tracking-wide uppercase" style="color: var(--color-text-muted)">
                {{ t('onboarding.preset') }}
              </label>
              <select
                :value="selectedPreset"
                @change="applyPreset(($event.target as HTMLSelectElement).value)"
                class="w-full h-9 px-3 rounded-lg text-sm outline-none transition-colors cursor-pointer"
                style="background: var(--color-surface); color: var(--color-text); border: 1px solid var(--color-border)"
              >
                <option value="">{{ t('onboarding.selectPreset') }}</option>
                <option v-for="p in providerPresets" :key="p.name" :value="p.name">
                  {{ p.name }}
                </option>
              </select>
            </div>

            <!-- Name -->
            <div class="mb-4">
              <label class="block text-xs font-medium mb-1.5 tracking-wide uppercase" style="color: var(--color-text-muted)">
                {{ t('onboarding.providerName') }}
              </label>
              <input
                v-model="providerForm.name"
                type="text"
                class="w-full h-9 px-3 rounded-lg text-sm outline-none transition-colors"
                style="background: var(--color-surface); color: var(--color-text); border: 1px solid var(--color-border)"
              />
            </div>

            <!-- API Key -->
            <div class="mb-4">
              <label class="block text-xs font-medium mb-1.5 tracking-wide uppercase" style="color: var(--color-text-muted)">
                {{ t('onboarding.apiKey') }}
              </label>
              <div class="relative">
                <input
                  v-model="providerForm.api_key"
                  :type="showApiKey ? 'text' : 'password'"
                  class="w-full h-9 pl-3 pr-9 rounded-lg text-sm outline-none transition-colors"
                  style="background: var(--color-surface); color: var(--color-text); border: 1px solid var(--color-border)"
                />
                <button
                  @click="showApiKey = !showApiKey"
                  class="absolute right-2 top-1/2 -translate-y-1/2 p-0.5 rounded opacity-40 hover:opacity-80 transition-opacity"
                  style="color: var(--color-text)"
                  tabindex="-1"
                >
                  <Eye v-if="showApiKey" :size="16" />
                  <EyeOff v-else :size="16" />
                </button>
              </div>
            </div>

            <!-- Base URL -->
            <div class="mb-4">
              <label class="block text-xs font-medium mb-1.5 tracking-wide uppercase" style="color: var(--color-text-muted)">
                {{ t('onboarding.baseUrl') }}
              </label>
              <input
                v-model="providerForm.base_url"
                type="text"
                class="w-full h-9 px-3 rounded-lg text-sm outline-none transition-colors"
                style="background: var(--color-surface); color: var(--color-text); border: 1px solid var(--color-border)"
              />
            </div>

            <!-- Error -->
            <p v-if="fetchError" class="text-xs mt-1" style="color: var(--color-danger)">
              {{ fetchError }}
            </p>

            <!-- Status -->
            <p v-if="isConnecting || isFetching" class="text-xs mt-3 flex items-center gap-1.5" style="color: var(--color-text-secondary)">
              <Loader2 :size="14" class="spin" style="color: var(--color-accent)" />
              {{ isConnecting ? t('onboarding.testingConnection') : t('onboarding.fetchingModels') }}
            </p>
          </div>

          <!-- Step 3: Select Models -->
          <div v-else-if="currentStep === 3" key="step3" class="flex flex-col py-6">
            <h2 class="text-lg font-medium mb-1" style="color: var(--color-text)">
              {{ t('onboarding.selectModelsTitle') }}
            </h2>
            <p class="text-sm mb-5" style="color: var(--color-text-secondary)">
              {{ t('onboarding.selectModelsBody') }}
            </p>

            <!-- Bulk actions -->
            <div class="flex gap-3 mb-4">
              <button
                @click="selectAll"
                class="text-xs font-medium px-3 py-1 rounded-md transition-colors"
                style="color: var(--color-accent); background: var(--color-accent-bg)"
              >
                {{ t('onboarding.selectAll') }}
              </button>
              <button
                @click="deselectAll"
                class="text-xs font-medium px-3 py-1 rounded-md transition-colors"
                style="color: var(--color-text-muted); background: var(--color-surface)"
              >
                {{ t('onboarding.deselectAll') }}
              </button>
            </div>

            <!-- Model list -->
            <div class="flex flex-col gap-1 max-h-56 overflow-y-auto pr-1">
              <label
                v-for="model in availableModels"
                :key="model"
                @click="toggleModel(model)"
                class="flex items-center gap-3 h-9 px-3 rounded-lg cursor-pointer transition-colors text-sm"
                :style="{
                  background: selectedModels.has(model) ? 'var(--color-accent-bg)' : 'transparent',
                  color: 'var(--color-text)',
                }"
              >
                <span
                  class="w-4 h-4 rounded flex items-center justify-center flex-shrink-0 transition-all"
                  :style="{
                    border: selectedModels.has(model) ? '1.5px solid var(--color-accent)' : '1.5px solid var(--color-border)',
                    background: selectedModels.has(model) ? 'var(--color-accent)' : 'transparent',
                  }"
                >
                  <Check v-if="selectedModels.has(model)" :size="10" :stroke-width="3" style="color: white" />
                </span>
                <span class="truncate">{{ model }}</span>
              </label>
            </div>

            <!-- Error -->
            <p v-if="fetchError" class="text-xs mt-3" style="color: var(--color-danger)">
              {{ fetchError }}
            </p>
          </div>

          <!-- Step 4: Done -->
          <div v-else-if="currentStep === 4" key="step4" class="flex flex-col items-center justify-center h-full py-10">
            <div class="w-12 h-12 rounded-full flex items-center justify-center mb-6" style="background: var(--color-success-bg)">
              <PartyPopper :size="22" style="color: var(--color-success)" />
            </div>
            <h2 class="text-xl font-medium mb-2" style="color: var(--color-text)">
              {{ t('onboarding.doneTitle') }}
            </h2>
            <p class="text-sm mb-4" style="color: var(--color-text-secondary)">
              {{ t('onboarding.doneBody') }}
            </p>
            <p class="text-xs px-3 py-1.5 rounded-md" style="color: var(--color-text-secondary); background: var(--color-surface)">
              {{ t('onboarding.shortcutHint', { shortcut: shortcutKey }) }}
            </p>
            <p class="text-xs mt-4" style="color: var(--color-text-muted)">
              {{ t('onboarding.addMoreProviders') }}
            </p>
          </div>

        </Transition>
      </div>

      <!-- Bottom navigation -->
      <div class="flex items-center justify-between py-6">
        <!-- Previous button -->
        <button
          v-if="currentStep > 0"
          @click="goPrev"
          class="flex items-center gap-1.5 h-9 px-4 rounded-lg text-sm font-medium transition-colors"
          style="color: var(--color-text-secondary)"
        >
          <ChevronLeft :size="16" />
          {{ t('onboarding.previous') }}
        </button>
        <div v-else />

        <!-- Step dots -->
        <div class="flex items-center gap-2">
          <span
            v-for="i in 5"
            :key="i"
            class="w-1.5 h-1.5 rounded-full transition-all duration-300"
            :style="{
              background: currentStep >= i - 1
                ? 'var(--color-accent)'
                : 'var(--color-border)',
              opacity: currentStep === i - 1 ? 1 : 0.5,
              transform: currentStep === i - 1 ? 'scale(1.4)' : 'scale(1)',
            }"
          />
        </div>

        <!-- Next button -->
        <button
          @click="goNext"
          :disabled="!canProceed || isConnecting || isFetching"
          class="flex items-center gap-1.5 h-9 px-5 rounded-lg text-sm font-medium transition-all"
          :style="{
            background: (!canProceed || isConnecting || isFetching) ? 'var(--color-surface)' : 'var(--color-accent)',
            color: (!canProceed || isConnecting || isFetching) ? 'var(--color-text-muted)' : 'white',
            cursor: (!canProceed || isConnecting || isFetching) ? 'not-allowed' : 'pointer',
          }"
        >
          <Loader2 v-if="isConnecting || isFetching" :size="14" class="spin" />
          <template v-else>
            {{ isLastStep ? t('onboarding.finish') : currentStep === 2 ? t('onboarding.testAndContinue') : t('onboarding.next') }}
            <ChevronRight v-if="!isLastStep" :size="16" />
          </template>
        </button>
      </div>

    </div>
  </div>
</template>

<style scoped>
.slide-left-enter-active,
.slide-left-leave-active,
.slide-right-enter-active,
.slide-right-leave-active {
  transition: transform 0.3s ease, opacity 0.3s ease;
}

.slide-left-enter-from {
  transform: translateX(30px);
  opacity: 0;
}

.slide-left-leave-to {
  transform: translateX(-30px);
  opacity: 0;
}

.slide-right-enter-from {
  transform: translateX(-30px);
  opacity: 0;
}

.slide-right-leave-to {
  transform: translateX(30px);
  opacity: 0;
}

.spin {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

select,
input {
  transition: border-color 0.15s ease;
}

select:focus,
input:focus {
  border-color: var(--color-accent) !important;
}

/* Custom scrollbar for model list */
div::-webkit-scrollbar {
  width: 4px;
}

div::-webkit-scrollbar-track {
  background: transparent;
}

div::-webkit-scrollbar-thumb {
  background: var(--color-scrollbar);
  border-radius: 2px;
}
</style>
