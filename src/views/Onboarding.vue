<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import {
  appConfig,
  loadProviderPresets,
  saveConfig as persistConfig,
} from "../stores/config";
import type { ProviderConfig, ProviderPreset } from "../stores/config";
import {
  testProviderConnection,
  fetchProviderModels,
} from "../services/llm-client";
import {
  ChevronRight,
  ChevronLeft,
  ChevronDown,
  Loader2,
  Check,
  Eye,
  EyeOff,
  Zap,
  PartyPopper,
  Link2,
  X,
} from "@lucide/vue";

const { t } = useI18n();
const router = useRouter();

// ── Step management ──
const currentStep = ref(0);
const direction = ref<"forward" | "backward">("forward");

// ── Step 0: App language ──
const appLanguageOptions = [
  { value: "en", label: "English" },
  { value: "zh-CN", label: "简体中文" },
];
const showAppLangMenu = ref(false);

function selectAppLang(lang: string) {
  appConfig.app_lang = lang;
  showAppLangMenu.value = false;
}

const currentAppLangLabel = computed(() => {
  return appLanguageOptions.find(o => o.value === appConfig.app_lang)?.label || "English";
});

const currentPresetLabel = computed(() => {
  if (selectedPreset.value === "Custom") return t('onboarding.custom');
  return selectedPreset.value || t('onboarding.selectPreset');
});

async function testKeyConnection() {
  if (!providerForm.value.api_key || !providerForm.value.base_url) return;
  isTestingKey.value = true;
  testKeyStatus.value = "";
  const result = await testProviderConnection(providerForm.value);
  testKeyStatus.value = result.ok ? "ok" : "fail";
  isTestingKey.value = false;
  setTimeout(() => { testKeyStatus.value = ""; }, 3000);
}

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
const showPresetMenu = ref(false);
const isTestingKey = ref(false);
const testKeyStatus = ref<"ok" | "fail" | "">("");
const showCloseConfirm = ref(false);

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
      return selectedModels.value.size > 0;
    case 4:
      return true;
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
  direction.value = "forward";
  currentStep.value++;
}

function goPrev() {
  if (currentStep.value === 0) return;
  direction.value = "backward";
  currentStep.value--;
}

// ── Step 0 logic: language is applied immediately via selectAppLang ──

// ── Step 2 logic ──
function applyPreset(presetName: string) {
  if (presetName === "Custom") {
    selectedPreset.value = "Custom";
    providerForm.value.name = "";
    providerForm.value.base_url = "";
    providerForm.value.preset = undefined;
    providerForm.value.api_format = undefined;
  } else {
    const preset = providerPresets.value.find((p) => p.name === presetName);
    if (!preset) return;
    selectedPreset.value = presetName;
    providerForm.value.name = preset.provider_name;
    providerForm.value.base_url = preset.base_url;
    providerForm.value.preset = presetName;
    providerForm.value.api_format = { ...preset.api_format };
  }
  showPresetMenu.value = false;
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
    fetchError.value = modelsResult.error || t('onboarding.noModelsFound');
    isFetching.value = false;
    return;
  }

  availableModels.value = modelsResult.models;
  isFetching.value = false;

  direction.value = "forward";
  currentStep.value = 3;
}

// ── Step 3 retry ──
async function retryFetchModels() {
  fetchError.value = "";
  isFetching.value = true;

  const modelsResult = await fetchProviderModels(providerForm.value);
  if (!modelsResult.ok || !modelsResult.models || modelsResult.models.length === 0) {
    fetchError.value = modelsResult.error || t('onboarding.noModelsFound');
    isFetching.value = false;
    return;
  }

  availableModels.value = modelsResult.models;
  isFetching.value = false;
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

  await persistConfig();
  await invoke("set_onboarding_complete");

  // Hide window — don't show anything after finishing
  await invoke("hide_main_window");

  router.replace("/");
}

// ── Click outside to close dropdowns ──
function onRootClick(e: MouseEvent) {
  const t = e.target as HTMLElement;
  if (!t.closest(".sel-wrap")) {
    showAppLangMenu.value = false;
    showPresetMenu.value = false;
  }
}

// ── Close button ──
function handleClose() {
  if (currentStep.value === 4) {
    finishOnboarding();
  } else {
    showCloseConfirm.value = true;
  }
}

function confirmClose() {
  showCloseConfirm.value = false;
  invoke("hide_main_window");
}

// ── Init ──
onMounted(async () => {
  // Ensure window is properly sized and visible for onboarding
  invoke("show_onboarding_window");
  try {
    providerPresets.value = await loadProviderPresets();
  } catch (err) {
    console.error("Failed to load presets:", err);
  }
});
</script>

<template>
  <div class="flex items-center justify-center min-h-dvh px-6 select-none" style="background: var(--color-bg)" data-tauri-drag-region @click="onRootClick">
    <div class="w-full max-w-[520px] flex flex-col relative" style="min-height: 480px">

      <!-- Close button -->
      <button
        class="absolute top-0 right-0 z-10 flex items-center justify-center w-8 h-8 rounded-lg transition-colors"
        style="color: var(--color-text-muted)"
        @click="handleClose"
        :title="t('common.hide')"
      >
        <X :size="18" :stroke-width="1.5" />
      </button>

      <!-- Close confirmation modal -->
      <Transition name="drop">
        <div v-if="showCloseConfirm" class="absolute inset-0 z-50 flex items-center justify-center" style="background: rgba(0,0,0,0.4); backdrop-filter: blur(4px)">
          <div class="rounded-xl p-6 mx-4 max-w-xs w-full" style="background: var(--color-surface); border: 1px solid var(--color-border)">
            <p class="text-sm mb-5" style="color: var(--color-text); line-height: 1.5">
              {{ t('onboarding.exitConfirm') }}
            </p>
            <div class="flex gap-2 justify-end">
              <button
                class="h-8 px-4 rounded-lg text-xs font-medium transition-colors"
                style="background: var(--color-surface); border: 1px solid var(--color-border); color: var(--color-text-secondary)"
                @click="showCloseConfirm = false"
              >
                {{ t('common.cancel') }}
              </button>
              <button
                class="h-8 px-4 rounded-lg text-xs font-medium transition-colors"
                style="background: var(--color-danger); color: white"
                @click="confirmClose"
              >
                {{ t('onboarding.exitAnyway') }}
              </button>
            </div>
          </div>
        </div>
      </Transition>

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
              <div class="sel-wrap" style="position: relative">
                <button class="sel-btn w-full" @click="showAppLangMenu = !showAppLangMenu">
                  <span class="sel-text">{{ currentAppLangLabel }}</span>
                  <ChevronDown :size="11" :stroke-width="2" class="sel-arrow" :class="{ rot: showAppLangMenu }" />
                </button>
                <Transition name="drop">
                  <div v-if="showAppLangMenu" class="sel-menu" style="position: absolute; top: 100%; left: 0; right: 0; margin-top: 4px; z-index: 10">
                    <div class="sel-clip">
                      <button
                        v-for="opt in appLanguageOptions" :key="opt.value"
                        class="sel-opt"
                        :class="{ hit: appConfig.app_lang === opt.value }"
                        @click="selectAppLang(opt.value)"
                      >
                        <div class="opt-info">
                          <span class="opt-id">{{ opt.label }}</span>
                        </div>
                        <Check v-if="appConfig.app_lang === opt.value" :size="13" :stroke-width="2.5" />
                      </button>
                    </div>
                  </div>
                </Transition>
              </div>
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
            <div class="mb-5">
              <label class="block text-xs font-medium mb-1.5 tracking-wide uppercase" style="color: var(--color-text-muted)">
                {{ t('onboarding.preset') }}
              </label>
              <div class="sel-wrap" style="position: relative">
                <button class="sel-btn w-full" @click="showPresetMenu = !showPresetMenu">
                  <span class="sel-text" :style="{ opacity: selectedPreset ? 1 : 0.5 }">{{ currentPresetLabel }}</span>
                  <ChevronDown :size="11" :stroke-width="2" class="sel-arrow" :class="{ rot: showPresetMenu }" />
                </button>
                <Transition name="drop">
                  <div v-if="showPresetMenu" class="sel-menu" style="position: absolute; top: 100%; left: 0; right: 0; margin-top: 4px; z-index: 10">
                    <div class="sel-clip">
                      <button
                        class="sel-opt"
                        :class="{ hit: selectedPreset === 'Custom' }"
                        @click="applyPreset('Custom')"
                      >
                        <div class="opt-info">
                          <span class="opt-id">{{ t('onboarding.custom') }}</span>
                        </div>
                        <Check v-if="selectedPreset === 'Custom'" :size="13" :stroke-width="2.5" />
                      </button>
                      <button
                        v-for="p in providerPresets" :key="p.name"
                        class="sel-opt"
                        :class="{ hit: selectedPreset === p.name }"
                        @click="applyPreset(p.name)"
                      >
                        <div class="opt-info">
                          <span class="opt-id">{{ p.name }}</span>
                          <span class="opt-src">{{ p.base_url }}</span>
                        </div>
                        <Check v-if="selectedPreset === p.name" :size="13" :stroke-width="2.5" />
                      </button>
                    </div>
                  </div>
                </Transition>
              </div>
            </div>

            <!-- Name -->
            <div class="mb-4">
              <label class="block text-xs font-medium mb-1.5 tracking-wide uppercase" style="color: var(--color-text-muted)">
                {{ t('onboarding.providerName') }}
              </label>
              <input
                v-model="providerForm.name"
                type="text"
                class="w-full h-9 px-3 rounded-lg text-sm outline-none transition-colors select-text"
                style="background: var(--color-surface); color: var(--color-text); border: 1px solid var(--color-border)"
              />
              <p v-if="!selectedPreset || selectedPreset === 'Custom'" class="mt-1.5" style="font-size: 10.5px; color: var(--color-text-muted); line-height: 1.4">
                {{ t('settings.openaiCompatHint') }}
              </p>
            </div>

            <!-- Base URL -->
            <div class="mb-4">
              <label class="block text-xs font-medium mb-1.5 tracking-wide uppercase" style="color: var(--color-text-muted)">
                {{ t('onboarding.baseUrl') }}
              </label>
              <input
                v-model="providerForm.base_url"
                type="text"
                class="w-full h-9 px-3 rounded-lg text-sm outline-none transition-colors select-text"
                style="background: var(--color-surface); color: var(--color-text); border: 1px solid var(--color-border)"
              />
            </div>

            <!-- API Key -->
            <div class="mb-4">
              <label class="block text-xs font-medium mb-1.5 tracking-wide uppercase" style="color: var(--color-text-muted)">
                {{ t('onboarding.apiKey') }}
              </label>
              <div class="flex gap-2">
                <div class="relative flex-1">
                  <input
                    v-model="providerForm.api_key"
                    :type="showApiKey ? 'text' : 'password'"
                    class="w-full h-9 pl-3 pr-9 rounded-lg text-sm outline-none transition-colors select-text"
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
                <button
                  class="flex items-center justify-center h-9 w-9 rounded-lg transition-colors"
                  :style="{
                    background: testKeyStatus === 'ok' ? 'var(--color-accent-bg)' : testKeyStatus === 'fail' ? 'rgba(239,68,68,0.1)' : 'var(--color-surface)',
                    border: '1px solid var(--color-border)',
                    cursor: (!providerForm.api_key || !providerForm.base_url || isTestingKey) ? 'not-allowed' : 'pointer',
                    opacity: (!providerForm.api_key || !providerForm.base_url) ? 0.4 : 1,
                  }"
                  :disabled="!providerForm.api_key || !providerForm.base_url || isTestingKey"
                  @click="testKeyConnection"
                  :title="t('settings.testConnection')"
                >
                  <Loader2 v-if="isTestingKey" :size="14" class="spin" style="color: var(--color-accent)" />
                  <Check v-else-if="testKeyStatus === 'ok'" :size="14" style="color: var(--color-accent)" />
                  <Link2 v-else :size="14" style="color: var(--color-text-muted)" />
                </button>
              </div>
              <p v-if="testKeyStatus === 'fail'" class="text-xs mt-1.5" style="color: var(--color-danger)">
                {{ t('onboarding.connectionFailed') }}
              </p>
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

            <!-- Error + Retry -->
            <div v-if="fetchError" class="flex items-center gap-2 mt-3">
              <p class="text-xs" style="color: var(--color-danger)">
                {{ fetchError }}
              </p>
              <button
                @click="retryFetchModels"
                :disabled="isFetching"
                class="text-xs font-medium px-3 py-1 rounded-md transition-colors"
                style="color: var(--color-accent); background: var(--color-accent-bg)"
              >
                {{ t('onboarding.retryFetch') }}
              </button>
            </div>
          </div>

          <!-- Step 4: Done -->
          <div v-else-if="currentStep === 4" key="step4" class="flex flex-col items-center justify-center h-full py-10">
            <div class="w-12 h-12 rounded-full flex items-center justify-center mb-6" style="background: var(--color-accent-bg)">
              <PartyPopper :size="22" style="color: var(--color-accent)" />
            </div>
            <h2 class="text-xl font-medium mb-2" style="color: var(--color-text)">
              {{ t('onboarding.doneTitle') }}
            </h2>
            <p class="text-sm mb-4 text-center max-w-xs" style="color: var(--color-text-secondary)">
              {{ t('onboarding.doneBody', { shortcut: shortcutKey }) }}
            </p>
            <p class="text-xs mt-4" style="color: var(--color-text-muted)">
              {{ t('onboarding.shortcutHint') }}
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
          style="background: var(--color-surface); border: 1px solid var(--color-border); color: var(--color-text-secondary)"
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
            {{ isLastStep ? t('onboarding.finish') : t('onboarding.next') }}
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

input {
  transition: border-color 0.15s ease;
}

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

/* ── Dropdown (matching Settings style) ── */
.sel-btn {
  display:flex; align-items:center; gap:8px; width:100%;
  padding: 9px 13px; border-radius:9px; font-size:12px;
  background: var(--color-surface); border: 1px solid var(--color-scrollbar);
  color: var(--color-text); cursor:pointer; transition:.15s; text-align:left;
}
.sel-btn:hover{ border-color: var(--color-border-hover); background: var(--color-surface); }
.sel-text {
  flex:1; font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
  font-size: 11.5px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;
}
.sel-arrow { color: var(--color-text-muted); transition: transform .18s; flex-shrink:0; }
.sel-arrow.rot{ transform: rotate(180deg); }
.sel-menu {
  min-width:200px; max-width:320px; max-height:180px;
  padding: 0; border-radius: 11px;
  background: var(--color-overlay); backdrop-filter: blur(20px) saturate(1.4);
  border: 1px solid var(--color-border);
  box-shadow: 0 16px 40px rgba(0,0,0,.55), 0 0 0 1px var(--color-surface);
  overflow:hidden;
}
.sel-clip{ max-height:inherit; overflow-y:auto; overflow-x:hidden; padding:5px 7px 5px 5px; }
.sel-opt {
  display:flex; align-items:center; justify-content:space-between; gap:10px;
  width:100%; padding: 8px 11px; border-radius:7px; font-size:11.5px;
  color: var(--color-text-secondary); cursor:pointer;
  border:none; background:none; text-align:left; transition:.1s;
}
.sel-opt:hover{ background: var(--color-surface-hover); color: var(--color-text); }
.sel-opt.hit{
  background: var(--color-accent-bg); color: var(--color-accent);
}
.opt-info{ display:flex; flex-direction:column; gap:1px; min-width:0; }
.opt-id{
  font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
  font-size: 11.5px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;
}
.opt-src{ font-size: 9px; color: var(--color-text-muted); letter-spacing: .02em; }
.drop-enter-active,.drop-leave-active{ transition:opacity .14s ease,transform .14s ease; }
.drop-enter-from,.drop-leave-to{ opacity:0; transform: translateY(-5px) scale(.967); }
</style>
