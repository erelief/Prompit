<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useRouter } from "vue-router";
import { useShortcutTriggered } from "../composables/useTauriEvents";
import { listen } from "@tauri-apps/api/event";
import { loadConfig, getActiveModel, appConfig } from "../stores/config";
import { translate } from "../services/llm-client";
import { Settings, LoaderCircle, Send, X, ClipboardPaste, ChevronDown, UserCircle } from "@lucide/vue";

const router = useRouter();

const inputText = ref("");
const translatedText = ref("");
const isLoading = ref(false);
const errorMessage = ref("");
const textareaRef = ref<HTMLTextAreaElement | null>(null);
const hasResult = ref(false);
const growAbove = ref(false);
const bodyHeight = ref(0);
let lastSentHeight = 0;
let resizeObserver: ResizeObserver | null = null;
let unlistenConfig: (() => void) | null = null;

const activeModelName = computed(() => {
  const m = getActiveModel();
  if (!m) return null;
  return m.model || null;
});

const showModelDropdown = ref(false);
const modelDropdownRef = ref<HTMLDivElement | null>(null);
const modelBtnRef = ref<HTMLButtonElement | null>(null);
const modelMenuRef = ref<HTMLDivElement | null>(null);
const dropdownPos = ref({ top: 0, left: 0 });

function toggleModelDropdown() {
  if (!showModelDropdown.value && modelBtnRef.value) {
    const rect = modelBtnRef.value.getBoundingClientRect();
    dropdownPos.value = { top: rect.bottom + 4, left: rect.left };
    showModelDropdown.value = true;
    nextTick(() => {
      if (modelMenuRef.value) {
        dropdownPos.value = {
          top: rect.top - modelMenuRef.value.offsetHeight - 4,
          left: rect.left,
        };
      }
    });
  } else {
    showModelDropdown.value = false;
  }
}

function selectModel(pIndex: number, mIndex: number) {
  appConfig.active_provider_index = pIndex;
  appConfig.active_model_index = mIndex;
  showModelDropdown.value = false;
}

// Flatten all provider models for dropdown: [{pIndex, mIndex, id}]
const allModels = computed(() => {
  const result: Array<{ pIndex: number; mIndex: number; id: string }> = [];
  appConfig.providers.forEach((prov, pi) => {
    prov.models.forEach((m, mi) => {
      result.push({ pIndex: pi, mIndex: mi, id: m.id });
    });
  });
  return result;
});

const isActiveModelEntry = (pIndex: number, mIndex: number) =>
  pIndex === appConfig.active_provider_index && mIndex === appConfig.active_model_index;

// ── Persona selector ──
const activePersonaName = computed(() => {
  const p = appConfig.personas.find((p) => p.enabled);
  return p?.name || null;
});

const showPersonaDropdown = ref(false);
const personaDropdownRef = ref<HTMLDivElement | null>(null);
const personaBtnRef = ref<HTMLButtonElement | null>(null);
const personaMenuRef = ref<HTMLDivElement | null>(null);
const personaDropdownPos = ref({ top: 0, left: 0 });

function togglePersonaDropdown() {
  if (!showPersonaDropdown.value && personaBtnRef.value) {
    const rect = personaBtnRef.value.getBoundingClientRect();
    personaDropdownPos.value = { top: rect.bottom + 4, left: rect.left };
    showPersonaDropdown.value = true;
    nextTick(() => {
      if (personaMenuRef.value) {
        personaDropdownPos.value = {
          top: rect.top - personaMenuRef.value.offsetHeight - 4,
          left: rect.left,
        };
      }
    });
  } else {
    showPersonaDropdown.value = false;
  }
}

function selectPersona(index: number) {
  const wasOn = appConfig.personas[index].enabled;
  for (const p of appConfig.personas) p.enabled = false;
  if (!wasOn) appConfig.personas[index].enabled = true;
  showPersonaDropdown.value = false;
}

function onDocumentClick(e: MouseEvent) {
  const target = e.target as Node;
  if (
    modelDropdownRef.value?.contains(target) ||
    modelMenuRef.value?.contains(target)
  ) {
    return;
  }
  showModelDropdown.value = false;

  if (
    personaDropdownRef.value?.contains(target) ||
    personaMenuRef.value?.contains(target)
  ) {
    return;
  }
  showPersonaDropdown.value = false;
}

watch(inputText, () => {
  if (hasResult.value) {
    hasResult.value = false;
    translatedText.value = "";
  }
});

function handleKeydown(e: KeyboardEvent) {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    if (hasResult.value) {
      handlePasteResult();
    } else {
      handleTranslate();
    }
  }
  if (e.key === "Escape") {
    handleHide();
  }
}

async function handleTranslate() {
  const text = inputText.value.trim();
  if (!text || isLoading.value) return;

  errorMessage.value = "";
  translatedText.value = "";
  isLoading.value = true;

  try {
    const result = await translate(text);
    translatedText.value = result;
    hasResult.value = true;
  } catch (err) {
    errorMessage.value = String(err);
  } finally {
    isLoading.value = false;
  }
}

async function handlePasteResult() {
  const text = translatedText.value;
  if (!text) return;

  await invoke("hide_main_window");
  await invoke("simulate_paste", { text });
  clearAll();
}

async function handleHide() {
  await invoke("hide_main_window");
}

async function handleDrag(e: MouseEvent) {
  // Only drag from the background, not from interactive elements
  const target = e.target as HTMLElement;
  if (target.closest("textarea, button, input, a, .model-dropdown, .persona-dropdown")) return;
  await getCurrentWindow().startDragging();
}

async function handleOpenSettings() {
  await invoke("open_settings_window");
  router.push("/settings");
}

function clearAll() {
  inputText.value = "";
  translatedText.value = "";
  errorMessage.value = "";
  hasResult.value = false;
}

onMounted(async () => {
  await loadConfig();
  document.addEventListener("mousedown", onDocumentClick);
  nextTick(() => {
    textareaRef.value?.focus();
  });

  // Listen for grow_above config from backend
  unlistenConfig = await listen<boolean>("window-config", (e) => {
    growAbove.value = e.payload;
  });

  // Track body height for dynamic window resize
  resizeObserver = new ResizeObserver((entries) => {
    bodyHeight.value = entries[0].contentRect.height;
  });
  resizeObserver.observe(document.body);
});

onUnmounted(() => {
  document.removeEventListener("mousedown", onDocumentClick);
  unlistenConfig?.();
  resizeObserver?.disconnect();
});

// Resize window when content grows
watch(bodyHeight, (h) => {
  if (h > lastSentHeight) {
    lastSentHeight = h;
    invoke("resize_and_reposition", { height: h });
  }
});

defineExpose({ clearAll });

useShortcutTriggered(() => {
  clearAll();
  lastSentHeight = 0;
  nextTick(() => {
    textareaRef.value?.focus();
  });
});
</script>

<template>
  <div
    @mousedown="handleDrag"
    class="w-full h-full flex items-center justify-center rounded-xl overflow-hidden"
    style="
      background: linear-gradient(
        135deg,
        rgba(15, 15, 20, 0.85) 0%,
        rgba(20, 20, 30, 0.8) 100%
      );
      backdrop-filter: blur(24px) saturate(1.5);
    "
  >
    <div class="w-full max-w-[560px] px-5 py-4 flex flex-col gap-3 overflow-y-auto max-h-full">
      <!-- Input area -->
      <textarea
        ref="textareaRef"
        v-model="inputText"
        @keydown="handleKeydown"
        :placeholder="hasResult ? 'Press Enter to paste result...' : 'Type to translate...'"
        rows="1"
        class="floating-input w-full resize-none text-[13px] leading-relaxed outline-none"
      />

      <!-- Toolbar: Send + Settings + Dismiss -->
      <div class="flex items-center gap-2">
        <button
          @click="handleTranslate"
          :disabled="!inputText.trim() || isLoading"
          class="send-btn"
          title="Translate (Enter)"
        >
          <LoaderCircle v-if="isLoading" :size="14" class="animate-spin" />
          <template v-else>
            <Send :size="13" />
            <span class="text-[12px] font-medium tracking-wide ml-1">Send</span>
          </template>
        </button>

        <div class="relative" ref="modelDropdownRef">
          <button
            v-if="activeModelName"
            ref="modelBtnRef"
            @click="toggleModelDropdown"
            class="model-btn"
            :class="{ active: showModelDropdown }"
          >
            <span class="truncate max-w-[120px]">{{ activeModelName }}</span>
            <ChevronDown :size="10" :stroke-width="2" class="shrink-0 transition-transform"
              :style="{ transform: showModelDropdown ? 'rotate(180deg)' : 'rotate(0)' }" />
          </button>

          <Teleport to="body">
            <Transition name="dropdown">
              <div
                v-if="showModelDropdown && allModels.length > 0"
                ref="modelMenuRef"
                class="model-dropdown"
                :style="{ top: dropdownPos.top + 'px', left: dropdownPos.left + 'px' }"
              >
                <button
                  v-for="entry in allModels"
                  :key="entry.pIndex + '-' + entry.mIndex"
                  @click="selectModel(entry.pIndex, entry.mIndex)"
                  class="model-option"
                  :class="{ selected: isActiveModelEntry(entry.pIndex, entry.mIndex) }"
                >
                  <span class="truncate">{{ entry.id }}</span>
                  <span v-if="isActiveModelEntry(entry.pIndex, entry.mIndex)" class="check-mark">&#10003;</span>
                </button>
              </div>
            </Transition>
          </Teleport>
        </div>

        <!-- Persona selector -->
        <div v-if="appConfig.personas.length > 0" class="relative" ref="personaDropdownRef">
          <button
            ref="personaBtnRef"
            @click="togglePersonaDropdown"
            class="model-btn persona-btn"
            :class="{ active: showPersonaDropdown, on: !!activePersonaName }"
          >
            <UserCircle :size="11" :stroke-width="1.8" />
            <span class="truncate max-w-[100px]">{{ activePersonaName || 'Persona' }}</span>
            <ChevronDown :size="10" :stroke-width="2" class="shrink-0 transition-transform"
              :style="{ transform: showPersonaDropdown ? 'rotate(180deg)' : 'rotate(0)' }" />
          </button>

          <Teleport to="body">
            <Transition name="dropdown">
              <div
                v-if="showPersonaDropdown"
                ref="personaMenuRef"
                class="model-dropdown persona-dropdown"
                :style="{ top: personaDropdownPos.top + 'px', left: personaDropdownPos.left + 'px' }"
              >
                <button
                  v-for="(persona, pi) in appConfig.personas"
                  :key="pi"
                  @click="selectPersona(pi)"
                  class="model-option"
                  :class="{ selected: persona.enabled }"
                >
                  <span class="truncate">{{ persona.name }}</span>
                  <span v-if="persona.enabled" class="check-mark">&#10003;</span>
                </button>
              </div>
            </Transition>
          </Teleport>
        </div>

        <div class="flex-1"></div>

        <button
          @click="handleOpenSettings"
          class="icon-btn"
          title="Settings"
        >
          <Settings :size="14" :stroke-width="1.8" />
        </button>

        <button @click="handleHide" class="icon-btn" title="Hide (Esc)">
          <X :size="14" :stroke-width="1.8" />
        </button>
      </div>

      <!-- Loading state -->
      <Transition name="fade">
        <div
          v-if="isLoading"
          class="flex items-center gap-2 text-[11px] text-white/40"
        >
          <span class="inline-block w-1.5 h-1.5 rounded-full bg-amber-400/60 animate-pulse"></span>
          Translating...
        </div>
      </Transition>

      <!-- Error -->
      <Transition name="fade">
        <div
          v-if="errorMessage"
          class="text-[11px] text-red-400/80 flex items-center gap-1.5"
        >
          <X :size="12" :stroke-width="2" />
          {{ errorMessage }}
        </div>
      </Transition>

      <!-- Result area -->
      <Transition name="fade">
        <div v-if="translatedText" class="result-block">
          <div class="result-text">{{ translatedText }}</div>
          <div class="result-actions">
            <button
              @click="handlePasteResult"
              class="paste-btn"
              title="Paste into active field (Enter)"
            >
              <ClipboardPaste :size="12" />
              <span>Paste Result</span>
            </button>
          </div>
        </div>
      </Transition>
    </div>
  </div>
</template>

<style scoped>
.floating-input {
  background: rgba(255, 255, 255, 0.06);
  color: rgba(255, 255, 255, 0.9);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 10px;
  padding: 9px 14px;
  transition: border-color 0.2s ease, box-shadow 0.2s ease;
  field-sizing: content;
  max-height: 200px;
  overflow-y: auto;
}

.floating-input::placeholder {
  color: rgba(255, 255, 255, 0.25);
}

.floating-input:focus {
  border-color: rgba(217, 160, 71, 0.35);
  box-shadow: 0 0 0 2px rgba(217, 160, 71, 0.08);
}

/* Send button */
.send-btn {
  display: inline-flex;
  align-items: center;
  height: 30px;
  padding: 0 14px 0 11px;
  border-radius: 8px;
  background: linear-gradient(135deg, #d4a048 0%, #c4922e 100%);
  color: #1a1a1a;
  transition: all 0.15s ease;
  flex-shrink: 0;
}

.send-btn:hover:not(:disabled) {
  background: linear-gradient(135deg, #ddb35a 0%, #d4a048 100%);
  transform: translateY(-0.5px);
  box-shadow: 0 4px 12px rgba(212, 160, 72, 0.25);
}

.send-btn:active:not(:disabled) {
  transform: translateY(0);
}

.send-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

/* Model selector button */
.model-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 30px;
  padding: 0 8px 0 10px;
  border-radius: 8px;
  font-size: 10px;
  font-weight: 500;
  color: rgba(255, 255, 255, 0.4);
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.06);
  transition: all 0.15s ease;
  flex-shrink: 0;
}

.model-btn:hover,
.model-btn.active {
  color: rgba(255, 255, 255, 0.65);
  background: rgba(255, 255, 255, 0.08);
  border-color: rgba(255, 255, 255, 0.12);
}

/* Persona button */
.persona-btn.on {
  color: rgba(217, 160, 71, 0.85);
  border-color: rgba(217, 160, 71, 0.18);
  background: rgba(217, 160, 71, 0.06);
}
.persona-btn.on:hover,
.persona-btn.on.active {
  color: rgba(217, 160, 71, 1);
  border-color: rgba(217, 160, 71, 0.3);
  background: rgba(217, 160, 71, 0.1);
}

/* Model dropdown */
.model-dropdown {
  position: fixed;
  min-width: 160px;
  max-width: 240px;
  padding: 3px;
  border-radius: 8px;
  background: rgba(22, 22, 30, 0.96);
  border: 1px solid rgba(255, 255, 255, 0.08);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.45), 0 0 0 1px rgba(255, 255, 255, 0.03);
  backdrop-filter: blur(16px);
  z-index: 9999;
  overflow: hidden;
}

.model-option {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  width: 100%;
  padding: 6px 10px;
  border-radius: 5px;
  font-size: 11px;
  color: rgba(255, 255, 255, 0.55);
  text-align: left;
  transition: all 0.1s ease;
}

.model-option:hover {
  background: rgba(255, 255, 255, 0.06);
  color: rgba(255, 255, 255, 0.85);
}

.model-option.selected {
  color: rgba(217, 160, 71, 0.9);
}

.check-mark {
  font-size: 10px;
  flex-shrink: 0;
}

/* Dropdown transition */
.dropdown-enter-active,
.dropdown-leave-active {
  transition: opacity 0.12s ease, transform 0.12s ease;
}

.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(-4px) scale(0.97);
}

/* Icon buttons (settings, dismiss) */
.icon-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 7px;
  color: rgba(255, 255, 255, 0.3);
  transition: all 0.15s ease;
  flex-shrink: 0;
}

.icon-btn:hover {
  color: rgba(255, 255, 255, 0.7);
  background: rgba(255, 255, 255, 0.06);
}

/* Result block */
.result-block {
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 10px;
  overflow: hidden;
}

.result-text {
  padding: 12px 14px;
  font-size: 13px;
  line-height: 1.65;
  color: rgba(255, 255, 255, 0.82);
  white-space: pre-wrap;
  word-break: break-word;
  overflow-y: auto;
}

.result-actions {
  display: flex;
  justify-content: flex-end;
  padding: 6px 8px;
  border-top: 1px solid rgba(255, 255, 255, 0.04);
}

/* Paste Result button */
.paste-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 26px;
  padding: 0 10px 0 8px;
  border-radius: 6px;
  font-size: 11px;
  font-weight: 500;
  letter-spacing: 0.02em;
  color: rgba(217, 160, 71, 0.85);
  background: rgba(217, 160, 71, 0.08);
  border: 1px solid rgba(217, 160, 71, 0.12);
  transition: all 0.15s ease;
}

.paste-btn:hover {
  background: rgba(217, 160, 71, 0.15);
  border-color: rgba(217, 160, 71, 0.25);
  color: rgba(217, 160, 71, 1);
}

.paste-btn:active {
  transform: scale(0.97);
}

/* Transitions */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>
