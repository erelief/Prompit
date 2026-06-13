<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useRouter } from "vue-router";
import { burstParticles, popElement } from "../utils/burstParticles";
import { useShortcutTriggered } from "../composables/useTauriEvents";
import { listen } from "@tauri-apps/api/event";
import { loadConfig, saveConfig, getActiveModel, appConfig, refreshDictStatus, historyStore, loadHistory, saveHistoryEntry, MODES, getCurrentMode, loadProviderPresets } from "../stores/config";
import type { ProviderPreset } from "../stores/config";
import ProviderIcon from "../components/icons/providers/ProviderIcon.vue";
import { translate } from "../services/llm-client";
import { Settings, LoaderCircle, Send, X, ClipboardPaste, ChevronDown, History } from "@lucide/vue";
import { isDark } from "../composables/useTheme";
import { useI18n } from "vue-i18n";
import TranslateToolbar from "../components/TranslateToolbar.vue";
const { t } = useI18n();

const router = useRouter();

const inputText = ref("");
const translatedText = ref("");
const isLoading = ref(false);
const errorMessage = ref("");
const textareaRef = ref<HTMLTextAreaElement | null>(null);
const hasResult = ref(false);
const isRestoringHistory = ref(false);
const growAbove = ref(false);
const chevronTransform = (open: boolean) =>
  `rotate(${open === growAbove.value ? 0 : 180}deg)`;
const contentWrapRef = ref<HTMLDivElement | null>(null);
const bodyHeight = ref(0);
let lastSentHeight = 0;
let resizeObserver: ResizeObserver | null = null;
let unlistenConfig: (() => void) | null = null;

// ── History browsing (terminal-style ↑↓) ──
const historyIndex = ref<number | null>(null);
let draftSnapshot: { input: string; output: string } | null = null;

const activeModelName = computed(() => {
  const m = getActiveModel();
  if (!m) return null;
  return m.model || null;
});

const glassBg = computed(() => {
  const o = (appConfig.floating_opacity ?? 90) / 100;
  if (o >= 1) {
    return isDark()
      ? "linear-gradient(135deg, rgb(15,15,20) 0%, rgb(20,20,30) 100%)"
      : "linear-gradient(135deg, rgb(255,255,255) 0%, rgb(245,245,250) 100%)";
  }
  return isDark()
    ? `linear-gradient(135deg, rgba(15,15,20,${o}) 0%, rgba(20,20,30,${o * 0.94}) 100%)`
    : `linear-gradient(135deg, rgba(255,255,255,${o}) 0%, rgba(245,245,250,${o * 0.94}) 100%)`;
});

const floatingAlpha = computed(() => (appConfig.floating_opacity ?? 90) / 100);

const showModelDropdown = ref(false);
const floatingPresets = ref<ProviderPreset[]>([]);
const modelDropdownRef = ref<HTMLDivElement | null>(null);
const modelBtnRef = ref<HTMLButtonElement | null>(null);
const modelMenuRef = ref<HTMLDivElement | null>(null);
const dropdownPos = ref({ top: 0, left: 0 });

function toggleModelDropdown() {
  showModeDropdown.value = false;
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
  const mode = appConfig.active_mode || "translate";
  (appConfig as any)[`${mode}_active_provider_index`] = pIndex;
  (appConfig as any)[`${mode}_active_model_index`] = mIndex;
  showModelDropdown.value = false;
  if (hasResult.value) {
    hasResult.value = false;
    translatedText.value = "";
  }
}

// Flatten all provider models for dropdown: [{pIndex, mIndex, id}]
const allModels = computed(() => {
  const result: Array<{ pIndex: number; mIndex: number; id: string; icon: string }> = [];
  appConfig.providers.forEach((prov, pi) => {
    prov.models.forEach((m, mi) => {
      result.push({
        pIndex: pi,
        mIndex: mi,
        id: m.id,
        icon: prov.preset ? (floatingPresets.value.find(p => p.name === prov.preset)?.icon ?? '') : '',
      });
    });
  });
  return result;
});

const isActiveModelEntry = (pIndex: number, mIndex: number) => {
  const mode = appConfig.active_mode || "translate";
  const config = appConfig as any;
  return pIndex === (config[`${mode}_active_provider_index`] ?? 0) && mIndex === (config[`${mode}_active_model_index`] ?? 0);
};

// ── Dropdown max-height (2 items visible, scroll beyond) ──
const ITEM_H = 28;
const PAD = 6;
const capHeight = (n: number) => n > 2 ? { maxHeight: `${2 * ITEM_H + PAD}px` } : {};
const modelDropdownStyle = computed(() => capHeight(allModels.value.length));

// ── Mode switch ──
const showModeDropdown = ref(false);
const modeBtnRef = ref<HTMLButtonElement | null>(null);
const modeMenuRef = ref<HTMLDivElement | null>(null);
const modeDropdownPos = ref({ top: 0, left: 0 });

const currentMode = computed(() => getCurrentMode());

function toggleModeDropdown() {
  showModelDropdown.value = false;
  if (!showModeDropdown.value && modeBtnRef.value) {
    const rect = modeBtnRef.value.getBoundingClientRect();
    modeDropdownPos.value = { top: rect.bottom + 4, left: rect.left };
    showModeDropdown.value = true;
    nextTick(() => {
      if (modeMenuRef.value) {
        const menuH = modeMenuRef.value.offsetHeight;
        const spaceBelow = window.innerHeight - rect.bottom - 4;
        const spaceAbove = rect.top - 4;
        if (menuH > spaceBelow && menuH <= spaceAbove) {
          modeDropdownPos.value = { top: rect.top - menuH - 4, left: rect.left };
        }
      }
    });
  } else {
    showModeDropdown.value = false;
  }
}

function selectMode(modeId: string) {
  appConfig.active_mode = modeId;
  showModeDropdown.value = false;
  if (modeBtnRef.value) {
    burstParticles(modeBtnRef.value);
    popElement(modeBtnRef.value);
  }
}

function onDocumentClick(e: MouseEvent) {
  const target = e.target as Node;
  if (modelDropdownRef.value?.contains(target) || modelMenuRef.value?.contains(target)) return;
  showModelDropdown.value = false;
  if (modeBtnRef.value?.contains(target) || modeMenuRef.value?.contains(target)) return;
  showModeDropdown.value = false;
}

watch(inputText, () => {
  if (isRestoringHistory.value) return;
  if (historyIndex.value !== null) {
    historyIndex.value = null;
    draftSnapshot = null;
  }
  if (hasResult.value) {
    hasResult.value = false;
    translatedText.value = "";
  }
});

function handleKeydown(e: KeyboardEvent) {
  // ── History navigation with ↑↓ ──
  if (e.key === "ArrowUp" && !e.shiftKey && !e.metaKey && !e.ctrlKey) {
    const ta = textareaRef.value;
    if (ta && ta.selectionStart === 0 && ta.selectionEnd === 0) {
      e.preventDefault();
      navigateHistory(1); // ↑ = toward older
      return;
    }
  }
  if (e.key === "ArrowDown" && !e.shiftKey && !e.metaKey && !e.ctrlKey) {
    const ta = textareaRef.value;
    if (ta && ta.selectionStart === ta.value.length && ta.selectionEnd === ta.value.length) {
      e.preventDefault();
      navigateHistory(-1); // ↓ = toward newer
      return;
    }
  }

  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    if (hasResult.value) {
      handlePasteResult();
    } else {
      handleTranslate();
    }
    historyIndex.value = null;
    draftSnapshot = null;
  }
  if (e.key === "Escape") {
    if (historyIndex.value !== null) {
      historyIndex.value = null;
      draftSnapshot = null;
    } else {
      handleHide();
    }
  }
}

function navigateHistory(direction: -1 | 1) {
  // direction: -1 = toward newer (index--), +1 = toward older (index++)
  const entries = historyStore.entries;
  if (entries.length === 0) return;

  let next: number;
  if (historyIndex.value === null) {
    next = direction === 1 ? 0 : -1;
  } else {
    next = historyIndex.value + direction;
  }

  // Save draft snapshot before first navigation
  if (historyIndex.value === null && direction === 1 && (inputText.value || translatedText.value)) {
    draftSnapshot = { input: inputText.value, output: translatedText.value };
  }

  // Going below index 0 (newer than newest) → restore draft or stay
  if (next < 0) {
    if (draftSnapshot) {
      historyIndex.value = null;
      isRestoringHistory.value = true;
      inputText.value = draftSnapshot.input;
      translatedText.value = draftSnapshot.output;
      hasResult.value = !!draftSnapshot.output;
      errorMessage.value = "";
      nextTick(() => { isRestoringHistory.value = false; });
      draftSnapshot = null;
    }
    return;
  }

  if (next >= entries.length) return;

  historyIndex.value = next;
  const entry = entries[next];
  isRestoringHistory.value = true;
  inputText.value = entry.input;
  translatedText.value = entry.output;
  hasResult.value = !!entry.output;
  errorMessage.value = "";
  nextTick(() => {
    isRestoringHistory.value = false;
    const ta = textareaRef.value;
    if (ta) {
      if (direction === 1) {
        // ↑ = cursor at start so next ↑ immediately navigates further
        ta.selectionStart = ta.selectionEnd = 0;
      } else {
        // ↓ = cursor at end so next ↓ immediately navigates further
        ta.selectionStart = ta.selectionEnd = ta.value.length;
      }
    }
  });
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
    await saveHistoryEntry(text, result);
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

function handleResultStale() {
  if (hasResult.value) {
    hasResult.value = false;
    translatedText.value = "";
  }
}

async function handleDrag(e: MouseEvent) {
  // Only drag from the background, not from interactive elements
  const target = e.target as HTMLElement;
  if (target.closest("textarea, button, input, a, .model-dropdown")) return;
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
  lastSentHeight = 0;

  await loadConfig();
  refreshDictStatus();
  await loadHistory();
  loadProviderPresets().then(p => { floatingPresets.value = p; }).catch(console.error);

  // Restore from history panel if applicable
  const restore = sessionStorage.getItem("history-restore");
  if (restore) {
    sessionStorage.removeItem("history-restore");
    try {
      const entry = JSON.parse(restore);
      isRestoringHistory.value = true;
      inputText.value = entry.input || "";
      translatedText.value = entry.output || "";
      hasResult.value = !!entry.output;
      nextTick(() => { isRestoringHistory.value = false; });
    } catch { /* ignore */ }
  }
  document.addEventListener("mousedown", onDocumentClick);
  nextTick(() => {
    textareaRef.value?.focus();
  });

  // Listen for grow_above config from backend
  unlistenConfig = await listen<boolean>("window-config", (e) => {
    growAbove.value = e.payload;
    nextTick(() => {
      textareaRef.value?.focus();
    });
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
  if (router.currentRoute.value.path !== '/') router.push('/');
});
</script>

<template>
  <div
    @mousedown="handleDrag"
    class="w-full h-full flex justify-center overflow-hidden"
    :class="growAbove ? 'items-end' : 'items-start'"
    :style="{ background: glassBg, backdropFilter: 'blur(24px) saturate(1.5)' }"
  >
    <div ref="contentWrapRef"
      class="w-full max-w-[560px] px-5 py-4 flex flex-col gap-1.5 overflow-y-auto flex-shrink-0 h-fit"
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
            class="flex items-center gap-2 text-[11px] text-[var(--color-text-secondary)]"
          >
            <span class="inline-block w-1.5 h-1.5 rounded-full bg-amber-400/60 animate-pulse"></span>
            {{ t('floating.sending') }}
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
          <div class="textarea-with-history">
            <textarea
              ref="textareaRef"
              v-model="inputText"
              @keydown="handleKeydown"
              :placeholder="hasResult ? t('floating.pressEnterToPaste') : t('floating.typeToSend')"
              rows="1"
              class="floating-input w-full resize-none text-[13px] leading-relaxed outline-none"
            ></textarea>

            <!-- History button (top-left corner of textarea) -->
            <button
              @click="router.push('/history')"
              class="history-btn"
              :title="t('floating.history')"
              :style="{ background: glassBg, '--btn-alpha': floatingAlpha }"
            >
              <History :size="14" />
            </button>
          </div>

          <button
            @click="hasResult ? handlePasteResult() : handleTranslate()"
            :disabled="(!inputText.trim() && !hasResult) || isLoading"
            class="send-btn-inline"
            :class="{ 'paste-mode': hasResult }"
            :title="hasResult ? t('floating.pasteIntoActiveField') : t('floating.send')"
          >
            <LoaderCircle v-if="isLoading" :size="14" class="animate-spin" />
            <ClipboardPaste v-else-if="hasResult" :size="13" />
            <Send v-else :size="13" />
          </button>
        </div>

        <!-- Toolbar -->
        <div class="flex items-center gap-2">
          <!-- Mode switch button -->
          <div class="relative">
            <button
              ref="modeBtnRef"
              @click="toggleModeDropdown"
              class="mode-btn"
              :class="{ active: showModeDropdown }"
              :title="t(currentMode.labelKey)"
            >
              <component :is="currentMode.icon" :size="14" :stroke-width="1.8" />
            </button>

            <Teleport to="body">
              <Transition name="dropdown">
                <div
                  v-if="showModeDropdown && MODES.length > 0"
                  ref="modeMenuRef"
                  class="model-dropdown"
                  :style="{ top: modeDropdownPos.top + 'px', left: modeDropdownPos.left + 'px' }"
                >
                  <button
                    v-for="mode in MODES"
                    :key="mode.id"
                    @click="selectMode(mode.id)"
                    class="model-option"
                    :class="{ selected: appConfig.active_mode === mode.id }"
                  >
                    <component :is="mode.icon" :size="12" :stroke-width="1.8" />
                    <span>{{ t(mode.labelKey) }}</span>
                    <span v-if="appConfig.active_mode === mode.id" class="check-mark">&#10003;</span>
                  </button>
                </div>
              </Transition>
            </Teleport>
          </div>

          <div class="w-px h-3 bg-[var(--color-border)] shrink-0"></div>

          <!-- Model selector -->
          <div class="relative" ref="modelDropdownRef">
            <button
              v-if="activeModelName"
              ref="modelBtnRef"
              @click="toggleModelDropdown"
              class="model-btn"
              :class="{ active: showModelDropdown }"
            >
              <span class="truncate max-w-[120px] min-w-0">{{ activeModelName }}</span>
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
                    <ProviderIcon :icon="entry.icon" :size="14" />
                    <span class="truncate">{{ entry.id }}</span>
                    <span v-if="isActiveModelEntry(entry.pIndex, entry.mIndex)" class="check-mark">&#10003;</span>
                  </button>
                </div>
              </Transition>
            </Teleport>
          </div>

          <div class="w-px h-3 bg-[var(--color-border)] shrink-0"></div>

          <!-- Mode-specific toolbar -->
          <TranslateToolbar :grow-above="growAbove" @result-stale="handleResultStale" />

          <div class="flex-1"></div>

          <button
            @click="handleOpenSettings"
            class="icon-btn"
            :title="t('common.settings')"
          >
            <Settings :size="14" :stroke-width="1.8" />
          </button>

          <button @click="handleHide" class="icon-btn" :title="t('common.hide')">
            <X :size="14" :stroke-width="1.8" />
          </button>
        </div>
      </template>

      <!-- !growAbove: result grows downward, input at top (default) -->
      <template v-else>
        <!-- Toolbar -->
        <div class="flex items-center gap-2">
          <!-- Mode switch button -->
          <div class="relative">
            <button
              ref="modeBtnRef"
              @click="toggleModeDropdown"
              class="mode-btn"
              :class="{ active: showModeDropdown }"
              :title="t(currentMode.labelKey)"
            >
              <component :is="currentMode.icon" :size="14" :stroke-width="1.8" />
            </button>

            <Teleport to="body">
              <Transition name="dropdown">
                <div
                  v-if="showModeDropdown && MODES.length > 0"
                  ref="modeMenuRef"
                  class="model-dropdown"
                  :style="{ top: modeDropdownPos.top + 'px', left: modeDropdownPos.left + 'px' }"
                >
                  <button
                    v-for="mode in MODES"
                    :key="mode.id"
                    @click="selectMode(mode.id)"
                    class="model-option"
                    :class="{ selected: appConfig.active_mode === mode.id }"
                  >
                    <component :is="mode.icon" :size="12" :stroke-width="1.8" />
                    <span>{{ t(mode.labelKey) }}</span>
                    <span v-if="appConfig.active_mode === mode.id" class="check-mark">&#10003;</span>
                  </button>
                </div>
              </Transition>
            </Teleport>
          </div>

          <div class="w-px h-3 bg-[var(--color-border)] shrink-0"></div>

          <!-- Model selector -->
          <div class="relative" ref="modelDropdownRef">
            <button
              v-if="activeModelName"
              ref="modelBtnRef"
              @click="toggleModelDropdown"
              class="model-btn"
              :class="{ active: showModelDropdown }"
            >
              <span class="truncate max-w-[120px] min-w-0">{{ activeModelName }}</span>
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
                    <ProviderIcon :icon="entry.icon" :size="14" />
                    <span class="truncate">{{ entry.id }}</span>
                    <span v-if="isActiveModelEntry(entry.pIndex, entry.mIndex)" class="check-mark">&#10003;</span>
                  </button>
                </div>
              </Transition>
            </Teleport>
          </div>

          <div class="w-px h-3 bg-[var(--color-border)] shrink-0"></div>

          <!-- Mode-specific toolbar -->
          <TranslateToolbar :grow-above="growAbove" @result-stale="handleResultStale" />

          <div class="flex-1"></div>

          <button
            @click="handleOpenSettings"
            class="icon-btn"
            :title="t('common.settings')"
          >
            <Settings :size="14" :stroke-width="1.8" />
          </button>

          <button @click="handleHide" class="icon-btn" :title="t('common.hide')">
            <X :size="14" :stroke-width="1.8" />
          </button>
        </div>

        <!-- Input area + inline send -->
        <div class="relative">
          <div class="textarea-with-history">
            <textarea
              ref="textareaRef"
              v-model="inputText"
              @keydown="handleKeydown"
              :placeholder="hasResult ? t('floating.pressEnterToPaste') : t('floating.typeToSend')"
              rows="1"
              class="floating-input w-full resize-none text-[13px] leading-relaxed outline-none"
            ></textarea>

            <!-- History button (top-left corner of textarea) -->
            <button
              @click="router.push('/history')"
              class="history-btn"
              :title="t('floating.history')"
              :style="{ background: glassBg, '--btn-alpha': floatingAlpha }"
            >
              <History :size="14" />
            </button>
          </div>

          <button
            @click="hasResult ? handlePasteResult() : handleTranslate()"
            :disabled="(!inputText.trim() && !hasResult) || isLoading"
            class="send-btn-inline"
            :class="{ 'paste-mode': hasResult }"
            :title="hasResult ? t('floating.pasteIntoActiveField') : t('floating.send')"
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
            class="flex items-center gap-2 text-[11px] text-[var(--color-text-secondary)]"
          >
            <span class="inline-block w-1.5 h-1.5 rounded-full bg-amber-400/60 animate-pulse"></span>
            {{ t('floating.sending') }}
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
  background: var(--color-surface);
  color: var(--color-text);
  border: 1px solid var(--color-border);
  border-radius: 10px;
  padding: 9px 36px 9px 14px;
  transition: border-color 0.2s ease, box-shadow 0.2s ease;
  field-sizing: content;
  max-height: 200px;
  overflow-y: auto;
  margin: 0;
}

.floating-input::placeholder {
  color: var(--color-text-placeholder);
}

.floating-input:focus {
  border-color: var(--color-accent-border);
  box-shadow: 0 0 0 2px var(--color-accent-bg);
}

/* Textarea with inline send button */
/* Keep old floating-input-with-btn class for potential future use */
.floating-input-with-btn {
  padding-right: 34px;
  border-top-right-radius: 8px;
  border-bottom-right-radius: 8px;
}

.floating-input-with-history {
  padding-left: 35px;
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
  background: linear-gradient(135deg, color-mix(in srgb, var(--color-accent) 85%, white) 0%, var(--color-accent) 100%);
  color: var(--color-bg);
  transition: all 0.15s ease;
  flex-shrink: 0;
  z-index: 3;
}

.send-btn-inline:hover:not(:disabled) {
  background: linear-gradient(135deg, color-mix(in srgb, var(--color-accent) 70%, white) 0%, color-mix(in srgb, var(--color-accent) 85%, white) 100%);
  box-shadow: 0 2px 8px var(--color-accent-bg);
}

.send-btn-inline:active:not(:disabled) {
  transform: scale(0.9);
}

.send-btn-inline:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.send-btn-inline.paste-mode {
  background: linear-gradient(135deg, color-mix(in srgb, var(--color-accent) 70%, white) 0%, color-mix(in srgb, var(--color-accent) 85%, white) 100%);
  box-shadow: 0 0 8px var(--color-accent-bg);
}

.send-btn-inline.paste-mode:hover:not(:disabled) {
  background: linear-gradient(135deg, color-mix(in srgb, var(--color-accent) 55%, white) 0%, color-mix(in srgb, var(--color-accent) 70%, white) 100%);
  box-shadow: 0 2px 10px var(--color-accent-bg);
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
  color: var(--color-text-muted);
  background: var(--color-surface);
  border: 1px solid var(--color-surface);
  transition: all 0.15s ease;
  flex-shrink: 0;
}

.model-btn:hover,
.model-btn.active {
  color: var(--color-text);
  background: var(--color-border);
  border-color: var(--color-border);
}

/* Model dropdown */
.toolbar-chevron {
  color: var(--color-text-muted);
  transition: transform 0.15s ease;
  flex-shrink: 0;
}

.model-dropdown {
  position: fixed;
  min-width: 160px;
  max-width: 240px;
  padding: 3px;
  border-radius: 8px;
  background: var(--color-overlay);
  border: 1px solid var(--color-border);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.45), 0 0 0 1px var(--color-surface);
  backdrop-filter: blur(16px);
  z-index: 9999;
  overflow-y: auto;
  overflow-x: hidden;
}
.model-dropdown::-webkit-scrollbar { width: 3px; }
.model-dropdown::-webkit-scrollbar-track { margin: 10px 0; }
.model-dropdown::-webkit-scrollbar-thumb { background: var(--color-border); border-radius: 3px; }

.model-option {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  width: 100%;
  padding: 6px 10px;
  border-radius: 5px;
  font-size: 11px;
  color: var(--color-text-secondary);
  text-align: left;
  transition: all 0.1s ease;
}

.model-option:hover {
  background: var(--color-surface);
  color: var(--color-text);
}

.model-option.selected {
  color: var(--color-accent);
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
  color: var(--color-text-muted);
  transition: all 0.15s ease;
  flex-shrink: 0;
}

.icon-btn:hover {
  color: var(--color-text);
  background: var(--color-surface);
}

/* Mode switch button — accent-colored to stand out */
.mode-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 7px;
  color: var(--color-accent);
  background: color-mix(in srgb, var(--color-accent) 12%, var(--color-surface));
  border: 1px solid color-mix(in srgb, var(--color-accent) 25%, transparent);
  transition: all 0.15s ease;
  flex-shrink: 0;
}
.mode-btn:hover {
  color: color-mix(in srgb, white 30%, var(--color-accent));
  background: color-mix(in srgb, var(--color-accent) 20%, var(--color-surface));
  border-color: color-mix(in srgb, var(--color-accent) 35%, transparent);
}
.mode-btn.active {
  background: color-mix(in srgb, var(--color-accent) 25%, var(--color-surface));
  border-color: var(--color-accent-border);
}

/* Result block */
.result-block {
  background: var(--color-accent-bg);
  border-left: 2.5px solid var(--color-accent);
  border-top: 1px solid var(--color-accent-border);
  border-right: 1px solid var(--color-accent-border);
  border-bottom: 1px solid var(--color-accent-border);
  border-radius: 8px;
  overflow: hidden;
}

.result-text {
  padding: 12px 14px;
  font-size: 13px;
  line-height: 1.65;
  color: var(--color-text);
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

.drop-enter-active,
.drop-leave-active {
  transition: opacity 0.14s ease, transform 0.14s ease;
}

.drop-enter-from,
.drop-leave-to {
  opacity: 0;
  transform: translateY(-5px) scale(0.967);
}

/* Thin scrollbar to prevent layout shift on appear */
:deep(.overflow-y-auto) {
  scrollbar-width: thin;
  scrollbar-color: var(--color-scrollbar) transparent;
}
:deep(.overflow-y-auto)::-webkit-scrollbar {
  width: 4px;
}
:deep(.overflow-y-auto)::-webkit-scrollbar-thumb {
  background: var(--color-scrollbar);
  border-radius: 4px;
}
:deep(.overflow-y-auto)::-webkit-scrollbar-track {
  background: transparent;
}

/* History button styling - exactly matches persona wand button pattern */
.textarea-with-history {
  position: relative !important;
}

.history-btn {
  position: absolute !important;
  top: -11px !important;
  left: -11px !important;
  width: 22px !important;
  height: 22px !important;
  border-radius: 50% !important;
  border: 1px solid var(--color-border) !important;
  color: var(--color-text-muted) !important;
  cursor: pointer !important;
  display: inline-flex !important;
  align-items: center !important;
  justify-content: center !important;
  opacity: 0 !important;
  transition: opacity 0.15s, color 0.15s, background 0.15s, border-color 0.15s !important;
  z-index: 9999 !important;
  box-shadow: 0 1px 3px rgba(0,0,0,.1) !important;
}

.textarea-with-history:hover .history-btn,
.history-btn.active {
  opacity: var(--btn-alpha, 1) !important;
}

.history-btn.active {
  color: var(--color-accent) !important;
  border-color: var(--color-accent) !important;
  background: color-mix(in srgb, var(--color-accent) 12%, var(--color-bg)) !important;
}

.history-btn:hover {
  color: var(--color-accent) !important;
  border-color: var(--color-accent-border) !important;
  background: color-mix(in srgb, var(--color-accent) 12%, var(--color-bg)) !important;
}

@media (prefers-reduced-motion: reduce) {
  .icon-btn.active { animation: none; }
}
</style>
