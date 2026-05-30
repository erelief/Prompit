<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useRouter } from "vue-router";
import { useShortcutTriggered } from "../composables/useTauriEvents";
import { listen } from "@tauri-apps/api/event";
import { loadConfig, saveConfig, getActiveModel, appConfig } from "../stores/config";
import { translate } from "../services/llm-client";
import { Settings, LoaderCircle, Send, X, ClipboardPaste, ChevronDown, UserCircle, Languages } from "@lucide/vue";

const router = useRouter();

const inputText = ref("");
const translatedText = ref("");
const isLoading = ref(false);
const errorMessage = ref("");
const textareaRef = ref<HTMLTextAreaElement | null>(null);
const hasResult = ref(false);
const growAbove = ref(false);
const chevronTransform = (open: boolean) =>
  `rotate(${open === growAbove.value ? 0 : 180}deg)`;
const contentWrapRef = ref<HTMLDivElement | null>(null);
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
  showLangDropdown.value = false;
  showPersonaDropdown.value = false;
  if (!showModelDropdown.value && modelBtnRef.value) {
    const rect = modelBtnRef.value.getBoundingClientRect();
    dropdownPos.value = { top: rect.bottom + 4, left: rect.left };
    showModelDropdown.value = true;
    nextTick(() => {
      if (modelMenuRef.value) {
        const menuH = modelMenuRef.value.offsetHeight;
        const spaceBelow = window.innerHeight - rect.bottom - 4;
        const spaceAbove = rect.top - 4;
        if (menuH > spaceBelow && menuH <= spaceAbove) {
          dropdownPos.value = { top: rect.top - menuH - 4, left: rect.left };
        }
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
  if (hasResult.value) {
    hasResult.value = false;
    translatedText.value = "";
  }
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
const lastActivePersonaIndex = ref(0);
const activePersonaName = computed(() => {
  const p = appConfig.personas.find((p) => p.enabled);
  return p?.name || null;
});
const personaOn = computed(() => appConfig.personas.some((p) => p.enabled));
const displayPersonaName = computed(() => {
  if (activePersonaName.value) return activePersonaName.value;
  const i = lastActivePersonaIndex.value;
  return i < appConfig.personas.length ? appConfig.personas[i].name : (appConfig.personas[0]?.name || 'Persona');
});

const showPersonaDropdown = ref(false);
const personaDropdownRef = ref<HTMLDivElement | null>(null);
const personaBtnRef = ref<HTMLButtonElement | null>(null);
const personaMenuRef = ref<HTMLDivElement | null>(null);
const personaDropdownPos = ref({ top: 0, left: 0 });

function togglePersona() {
  const active = appConfig.personas.findIndex((p) => p.enabled);
  if (active >= 0) {
    appConfig.personas[active].enabled = false;
  } else {
    const i = lastActivePersonaIndex.value < appConfig.personas.length
      ? lastActivePersonaIndex.value : 0;
    appConfig.personas[i].enabled = true;
  }
  if (hasResult.value) {
    hasResult.value = false;
    translatedText.value = "";
  }
}

function togglePersonaDropdown() {
  showModelDropdown.value = false;
  showLangDropdown.value = false;
  if (!showPersonaDropdown.value && personaBtnRef.value) {
    const rect = personaBtnRef.value.getBoundingClientRect();
    const wrapLeft = personaDropdownRef.value?.getBoundingClientRect().left ?? rect.left;
    personaDropdownPos.value = { top: rect.bottom + 4, left: wrapLeft };
    showPersonaDropdown.value = true;
    nextTick(() => {
      if (personaMenuRef.value) {
        const menuH = personaMenuRef.value.offsetHeight;
        const spaceBelow = window.innerHeight - rect.bottom - 4;
        const spaceAbove = rect.top - 4;
        if (menuH > spaceBelow && menuH <= spaceAbove) {
          personaDropdownPos.value = { top: rect.top - menuH - 4, left: wrapLeft };
        }
      }
    });
  } else {
    showPersonaDropdown.value = false;
  }
}

function selectPersona(index: number) {
  for (const p of appConfig.personas) p.enabled = false;
  appConfig.personas[index].enabled = true;
  lastActivePersonaIndex.value = index;
  showPersonaDropdown.value = false;
  if (hasResult.value) {
    hasResult.value = false;
    translatedText.value = "";
  }
}

// ── Language selector ──
const langCodeMap: Record<string, string> = {
  "English": "EN", "Simplified Chinese": "简中", "Traditional Chinese": "繁中",
  "Japanese": "JA", "Korean": "KO", "French": "FR",
  "German": "DE", "Spanish": "ES", "Russian": "RU",
};
const langCode = computed(() => langCodeMap[appConfig.target_lang] || appConfig.target_lang?.slice(0, 2).toUpperCase() || "EN");
const showLangDropdown = ref(false);
const langDropdownRef = ref<HTMLDivElement | null>(null);
const langBtnRef = ref<HTMLButtonElement | null>(null);
const langMenuRef = ref<HTMLDivElement | null>(null);
const langDropdownPos = ref({ top: 0, left: 0 });
const targetLanguages = ["English", "Simplified Chinese", "Traditional Chinese", "Japanese", "Korean", "French", "German", "Spanish", "Russian"];

function toggleLangDropdown() {
  showModelDropdown.value = false;
  showPersonaDropdown.value = false;
  if (!showLangDropdown.value && langBtnRef.value) {
    const rect = langBtnRef.value.getBoundingClientRect();
    langDropdownPos.value = { top: rect.bottom + 4, left: rect.left };
    showLangDropdown.value = true;
    nextTick(() => {
      if (langMenuRef.value) {
        const menuH = langMenuRef.value.offsetHeight;
        const spaceBelow = window.innerHeight - rect.bottom - 4;
        const spaceAbove = rect.top - 4;
        if (menuH > spaceBelow && menuH <= spaceAbove) {
          langDropdownPos.value = { top: rect.top - menuH - 4, left: rect.left };
        }
      }
    });
  } else {
    showLangDropdown.value = false;
  }
}

function pickLang(lang: string) {
  appConfig.target_lang = lang;
  showLangDropdown.value = false;
}

// ── Dropdown max-height (2 items visible, scroll beyond) ──
const ITEM_H = 28;
const PAD = 6;
const capHeight = (n: number) => n > 2 ? { maxHeight: `${2 * ITEM_H + PAD}px` } : {};
const modelDropdownStyle = computed(() => capHeight(allModels.value.length));
const personaDropdownStyle = computed(() => capHeight(appConfig.personas.length));
const langDropdownStyle = computed(() => capHeight(targetLanguages.length));

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

  if (
    langDropdownRef.value?.contains(target) ||
    langMenuRef.value?.contains(target)
  ) {
    return;
  }
  showLangDropdown.value = false;
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

  // Track content container height for dynamic window resize
  // Use scrollHeight — returns true content size even when overflow-clipped or flex-constrained
  resizeObserver = new ResizeObserver(() => {
    if (contentWrapRef.value) {
      bodyHeight.value = Math.ceil(contentWrapRef.value.scrollHeight);
    }
  });
  if (contentWrapRef.value) {
    resizeObserver.observe(contentWrapRef.value);
  }
});

// Auto-save config changes to disk
let saveTimer: ReturnType<typeof setTimeout> | null = null;
watch(() => JSON.stringify(appConfig), () => {
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(() => { saveConfig(); }, 800);
});

onUnmounted(() => {
  document.removeEventListener("mousedown", onDocumentClick);
  unlistenConfig?.();
  resizeObserver?.disconnect();
});

// Resize window when content changes
watch(bodyHeight, (h) => {
  if (h !== lastSentHeight) {
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
    class="w-full h-full flex justify-center overflow-hidden"
    :class="growAbove ? 'items-end' : 'items-start'"
    style="
      background: linear-gradient(
        135deg,
        rgba(15, 15, 20, 0.85) 0%,
        rgba(20, 20, 30, 0.8) 100%
      );
      backdrop-filter: blur(24px) saturate(1.5);
    "
  >
    <div ref="contentWrapRef"
      class="w-full max-w-[560px] px-5 py-4 flex flex-col gap-3 overflow-y-auto flex-shrink-0 h-fit"
      :class="{ 'justify-end': growAbove }">
      <!-- growAbove: result grows upward, input anchored at bottom -->
      <template v-if="growAbove">
        <!-- Result area -->
        <Transition name="fade">
          <div v-show="translatedText" class="result-block">
            <div class="result-text">{{ translatedText }}</div>
          </div>
        </Transition>

        <!-- Loading state -->
        <Transition name="fade">
          <div
            v-show="isLoading"
            class="flex items-center gap-2 text-[11px] text-white/40"
          >
            <span class="inline-block w-1.5 h-1.5 rounded-full bg-amber-400/60 animate-pulse"></span>
            Sending...
          </div>
        </Transition>

        <!-- Error -->
        <Transition name="fade">
          <div
            v-show="errorMessage"
            class="text-[11px] text-red-400/80 flex items-center gap-1.5"
          >
            <X :size="12" :stroke-width="2" />
            {{ errorMessage }}
          </div>
        </Transition>

        <!-- Input area + inline send -->
        <div class="relative">
          <textarea
            ref="textareaRef"
            v-model="inputText"
            @keydown="handleKeydown"
            :placeholder="hasResult ? 'Press Enter to paste result...' : 'Type to send...'"
            rows="1"
            class="floating-input floating-input-with-btn w-full resize-none text-[13px] leading-relaxed outline-none"
          />
          <button
            @click="hasResult ? handlePasteResult() : handleTranslate()"
            :disabled="(!inputText.trim() && !hasResult) || isLoading"
            class="send-btn-inline"
            :class="{ 'paste-mode': hasResult }"
            :title="hasResult ? 'Paste into active field (Enter)' : 'Send (Enter)'"
          >
            <LoaderCircle v-if="isLoading" :size="14" class="animate-spin" />
            <ClipboardPaste v-else-if="hasResult" :size="13" />
            <Send v-else :size="13" />
          </button>
        </div>

        <!-- Toolbar -->
        <div class="flex items-center gap-2">
          <!-- Model selector -->
          <div class="relative" ref="modelDropdownRef">
            <button
              v-if="activeModelName"
              ref="modelBtnRef"
              @click="toggleModelDropdown"
              class="model-btn"
              :class="{ active: showModelDropdown }"
            >
              <span class="truncate max-w-[120px]">{{ activeModelName }}</span>
              <ChevronDown :size="10" :stroke-width="2" class="toolbar-chevron"
                :style="{ transform: chevronTransform(showModelDropdown) }" />
            </button>

            <Teleport to="body">
              <Transition name="dropdown">
                <div
                  v-if="showModelDropdown && allModels.length > 0"
                  ref="modelMenuRef"
                  class="model-dropdown"
                  :style="{ top: dropdownPos.top + 'px', left: dropdownPos.left + 'px', ...modelDropdownStyle }"
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

          <div class="w-px h-3 bg-white/10"></div>

          <!-- Language selector -->
          <div class="lang-wrap" ref="langDropdownRef">
            <button
              ref="langBtnRef"
              @click="toggleLangDropdown"
              class="lang-btn"
              :class="{ active: showLangDropdown }"
              title="Target Language"
            >
              <Languages :size="11" :stroke-width="1.8" />
              <span>{{ langCode }}</span>
              <ChevronDown :size="10" :stroke-width="2" class="toolbar-chevron"
                :style="{ transform: chevronTransform(showLangDropdown) }" />
            </button>

            <Teleport to="body">
              <Transition name="dropdown">
                <div
                  v-if="showLangDropdown"
                  ref="langMenuRef"
                  class="model-dropdown lang-dropdown"
                  :style="{ top: langDropdownPos.top + 'px', left: langDropdownPos.left + 'px', ...langDropdownStyle }"
                >
                  <button
                    v-for="lang in targetLanguages"
                    :key="lang"
                    @click="pickLang(lang)"
                    class="model-option"
                    :class="{ selected: appConfig.target_lang === lang }"
                  >
                    <span class="truncate">{{ lang }}</span>
                    <span v-if="appConfig.target_lang === lang" class="check-mark">&#10003;</span>
                  </button>
                </div>
              </Transition>
            </Teleport>
          </div>

          <!-- Persona toggle + selector -->
          <div v-if="appConfig.personas.length > 0" class="persona-wrap" ref="personaDropdownRef">
            <button
              @click="togglePersona"
              class="persona-toggle"
              :class="{ on: personaOn }"
              :title="personaOn ? 'Disable persona' : 'Enable persona'"
            >
              <UserCircle :size="11" :stroke-width="1.8" />
              <span v-if="personaOn" class="persona-dot on" />
              <span class="truncate max-w-[90px]">{{ personaOn ? displayPersonaName : '' }}</span>
            </button>
            <button
              v-if="appConfig.personas.length > 1"
              ref="personaBtnRef"
              @click="togglePersonaDropdown"
              class="persona-chevron"
              :class="{ on: personaOn, active: showPersonaDropdown }"
            >
              <ChevronDown :size="10" :stroke-width="2" class="toolbar-chevron"
                :style="{ transform: chevronTransform(showPersonaDropdown) }" />
            </button>

            <Teleport to="body">
              <Transition name="dropdown">
                <div
                  v-if="showPersonaDropdown"
                  ref="personaMenuRef"
                  class="model-dropdown persona-dropdown"
                  :style="{ top: personaDropdownPos.top + 'px', left: personaDropdownPos.left + 'px', ...personaDropdownStyle }"
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
      </template>

      <!-- !growAbove: result grows downward, input at top (default) -->
      <template v-else>
        <!-- Toolbar -->
        <div class="flex items-center gap-2">
          <!-- Model selector -->
          <div class="relative" ref="modelDropdownRef">
            <button
              v-if="activeModelName"
              ref="modelBtnRef"
              @click="toggleModelDropdown"
              class="model-btn"
              :class="{ active: showModelDropdown }"
            >
              <span class="truncate max-w-[120px]">{{ activeModelName }}</span>
              <ChevronDown :size="10" :stroke-width="2" class="toolbar-chevron"
                :style="{ transform: chevronTransform(showModelDropdown) }" />
            </button>

            <Teleport to="body">
              <Transition name="dropdown">
                <div
                  v-if="showModelDropdown && allModels.length > 0"
                  ref="modelMenuRef"
                  class="model-dropdown"
                  :style="{ top: dropdownPos.top + 'px', left: dropdownPos.left + 'px', ...modelDropdownStyle }"
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

          <div class="w-px h-3 bg-white/10"></div>

          <!-- Language selector -->
          <div class="lang-wrap" ref="langDropdownRef">
            <button
              ref="langBtnRef"
              @click="toggleLangDropdown"
              class="lang-btn"
              :class="{ active: showLangDropdown }"
              title="Target Language"
            >
              <Languages :size="11" :stroke-width="1.8" />
              <span>{{ langCode }}</span>
              <ChevronDown :size="10" :stroke-width="2" class="toolbar-chevron"
                :style="{ transform: chevronTransform(showLangDropdown) }" />
            </button>

            <Teleport to="body">
              <Transition name="dropdown">
                <div
                  v-if="showLangDropdown"
                  ref="langMenuRef"
                  class="model-dropdown lang-dropdown"
                  :style="{ top: langDropdownPos.top + 'px', left: langDropdownPos.left + 'px', ...langDropdownStyle }"
                >
                  <button
                    v-for="lang in targetLanguages"
                    :key="lang"
                    @click="pickLang(lang)"
                    class="model-option"
                    :class="{ selected: appConfig.target_lang === lang }"
                  >
                    <span class="truncate">{{ lang }}</span>
                    <span v-if="appConfig.target_lang === lang" class="check-mark">&#10003;</span>
                  </button>
                </div>
              </Transition>
            </Teleport>
          </div>

          <!-- Persona toggle + selector -->
          <div v-if="appConfig.personas.length > 0" class="persona-wrap" ref="personaDropdownRef">
            <button
              @click="togglePersona"
              class="persona-toggle"
              :class="{ on: personaOn }"
              :title="personaOn ? 'Disable persona' : 'Enable persona'"
            >
              <UserCircle :size="11" :stroke-width="1.8" />
              <span v-if="personaOn" class="persona-dot on" />
              <span class="truncate max-w-[90px]">{{ personaOn ? displayPersonaName : '' }}</span>
            </button>
            <button
              v-if="appConfig.personas.length > 1"
              ref="personaBtnRef"
              @click="togglePersonaDropdown"
              class="persona-chevron"
              :class="{ on: personaOn, active: showPersonaDropdown }"
            >
              <ChevronDown :size="10" :stroke-width="2" class="toolbar-chevron"
                :style="{ transform: chevronTransform(showPersonaDropdown) }" />
            </button>

            <Teleport to="body">
              <Transition name="dropdown">
                <div
                  v-if="showPersonaDropdown"
                  ref="personaMenuRef"
                  class="model-dropdown persona-dropdown"
                  :style="{ top: personaDropdownPos.top + 'px', left: personaDropdownPos.left + 'px', ...personaDropdownStyle }"
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

        <!-- Input area + inline send -->
        <div class="relative">
          <textarea
            ref="textareaRef"
            v-model="inputText"
            @keydown="handleKeydown"
            :placeholder="hasResult ? 'Press Enter to paste result...' : 'Type to send...'"
            rows="1"
            class="floating-input floating-input-with-btn w-full resize-none text-[13px] leading-relaxed outline-none"
          />
          <button
            @click="hasResult ? handlePasteResult() : handleTranslate()"
            :disabled="(!inputText.trim() && !hasResult) || isLoading"
            class="send-btn-inline"
            :class="{ 'paste-mode': hasResult }"
            :title="hasResult ? 'Paste into active field (Enter)' : 'Send (Enter)'"
          >
            <LoaderCircle v-if="isLoading" :size="14" class="animate-spin" />
            <ClipboardPaste v-else-if="hasResult" :size="13" />
            <Send v-else :size="13" />
          </button>
        </div>

        <!-- Loading state -->
        <Transition name="fade">
          <div
            v-show="isLoading"
            class="flex items-center gap-2 text-[11px] text-white/40"
          >
            <span class="inline-block w-1.5 h-1.5 rounded-full bg-amber-400/60 animate-pulse"></span>
            Sending...
          </div>
        </Transition>

        <!-- Error -->
        <Transition name="fade">
          <div
            v-show="errorMessage"
            class="text-[11px] text-red-400/80 flex items-center gap-1.5"
          >
            <X :size="12" :stroke-width="2" />
            {{ errorMessage }}
          </div>
        </Transition>

        <!-- Result area -->
        <Transition name="fade">
          <div v-show="translatedText" class="result-block">
            <div class="result-text">{{ translatedText }}</div>
          </div>
        </Transition>
      </template>
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

/* Textarea with inline send button */
.floating-input-with-btn {
  padding-right: 34px;
  border-top-right-radius: 8px;
  border-bottom-right-radius: 8px;
}

.send-btn-inline {
  position: absolute;
  right: 7px;
  top: 7px;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: 6px;
  background: linear-gradient(135deg, #d4a048 0%, #c4922e 100%);
  color: #1a1a1a;
  transition: all 0.15s ease;
  flex-shrink: 0;
}

.send-btn-inline:hover:not(:disabled) {
  background: linear-gradient(135deg, #ddb35a 0%, #d4a048 100%);
  box-shadow: 0 2px 8px rgba(212, 160, 72, 0.3);
}

.send-btn-inline:active:not(:disabled) {
  transform: scale(0.9);
}

.send-btn-inline:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.send-btn-inline.paste-mode {
  background: linear-gradient(135deg, #e8b84a 0%, #d4a048 100%);
  box-shadow: 0 0 8px rgba(232, 184, 74, 0.35);
}

.send-btn-inline.paste-mode:hover:not(:disabled) {
  background: linear-gradient(135deg, #f0c55e 0%, #e8b84a 100%);
  box-shadow: 0 2px 10px rgba(232, 184, 74, 0.5);
}

/* Model selector button */
.model-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 28px;
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

/* Language selector */
.lang-wrap { display: inline-flex; flex-shrink: 0; }
.lang-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 28px;
  padding: 0 8px 0 7px;
  border-radius: 8px;
  font-size: 9.5px;
  font-weight: 600;
  letter-spacing: .04em;
  color: rgba(255, 255, 255, 0.3);
  background: rgba(255, 255, 255, 0.035);
  border: 1px solid rgba(255, 255, 255, 0.06);
  transition: all 0.15s ease;
}
.lang-btn:hover,
.lang-btn.active {
  color: rgba(255, 255, 255, 0.6);
  background: rgba(255, 255, 255, 0.07);
  border-color: rgba(255, 255, 255, 0.11);
}
.toolbar-chevron {
  color: rgba(255, 255, 255, 0.28);
  transition: transform 0.15s ease;
  flex-shrink: 0;
}

/* Persona toggle */
.persona-wrap {
  display: inline-flex;
  align-items: center;
  gap: 0;
  flex-shrink: 0;
}
.persona-toggle {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 28px;
  padding: 0 4px 0 8px;
  border-radius: 8px 0 0 8px;
  font-size: 10px;
  font-weight: 550;
  color: rgba(255, 255, 255, 0.28);
  background: rgba(255, 255, 255, 0.035);
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-right: none;
  transition: all 0.18s ease;
}
.persona-toggle.on { padding-right: 10px; }
.persona-toggle:hover {
  color: rgba(255, 255, 255, 0.5);
  background: rgba(255, 255, 255, 0.065);
}
.persona-toggle.on {
  color: rgba(217, 160, 71, 0.9);
  background: rgba(217, 160, 71, 0.07);
  border-color: rgba(217, 160, 71, 0.18);
}
.persona-toggle.on:hover {
  color: rgba(217, 160, 71, 1);
  background: rgba(217, 160, 71, 0.11);
}

/* Status dot */
.persona-dot {
  width: 5px; height: 5px; border-radius: 50%;
  background: rgba(217, 160, 71, 0.9);
  box-shadow: 0 0 5px rgba(217, 160, 71, 0.3);
  flex-shrink: 0;
}

/* Persona chevron */
.persona-chevron {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 28px;
  border-radius: 0 8px 8px 0;
  color: rgba(255, 255, 255, 0.22);
  background: rgba(255, 255, 255, 0.035);
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-left: 1px solid rgba(255, 255, 255, 0.04);
  transition: all 0.15s ease;
}
.persona-chevron:hover,
.persona-chevron.active {
  color: rgba(255, 255, 255, 0.5);
  background: rgba(255, 255, 255, 0.065);
  border-color: rgba(255, 255, 255, 0.1);
}
.persona-chevron.on {
  border-color: rgba(217, 160, 71, 0.18);
  background: rgba(217, 160, 71, 0.07);
}
.persona-chevron.on:hover,
.persona-chevron.on.active {
  border-color: rgba(217, 160, 71, 0.3);
  background: rgba(217, 160, 71, 0.11);
  color: rgba(217, 160, 71, 0.9);
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
  overflow-y: auto;
  overflow-x: hidden;
}
.model-dropdown::-webkit-scrollbar { width: 3px; }
.model-dropdown::-webkit-scrollbar-track { margin: 10px 0; }
.model-dropdown::-webkit-scrollbar-thumb { background: rgba(255,255,255,.12); border-radius: 3px; }

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

/* Transitions */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.15s ease-out;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

/* Thin scrollbar to prevent layout shift on appear */
:deep(.overflow-y-auto) {
  scrollbar-width: thin;
  scrollbar-color: rgba(255, 255, 255, 0.15) transparent;
}
:deep(.overflow-y-auto)::-webkit-scrollbar {
  width: 4px;
}
:deep(.overflow-y-auto)::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.15);
  border-radius: 4px;
}
:deep(.overflow-y-auto)::-webkit-scrollbar-track {
  background: transparent;
}
</style>
