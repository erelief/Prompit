<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useRouter } from "vue-router";
import { burstParticles, popElement } from "../utils/burstParticles";
import { eventMatchesShortcut } from "../utils/shortcut";
import { useShortcutTriggered } from "../composables/useTauriEvents";
import { listen } from "@tauri-apps/api/event";
import { MAIN_WIDTH } from "../composables/useSettingsWindow";
import { getActiveModel, appConfig, flushConfigSave, refreshDictStatus, historyStore, loadHistory, saveHistoryEntry, MODES, getCurrentMode, loadProviderPresets, getProviderIcon, sparkleStore } from "../stores/config";
import type { ProviderPreset, ModelInputCapabilities } from "../stores/config";
import ProviderIcon from "../components/icons/providers/ProviderIcon.vue";
import ModelCapabilityIcon from "../components/ModelCapabilityIcon.vue";
import { translate } from "../services/llm-client";
import type { TranslateOutcome } from "../services/llm-client";
import { SearchFailureError, ModelHttpError } from "../services/llm-client";
import { classifySearchError } from "../services/websearch";
import type { SearchHit } from "../services/websearch/types";
import { Settings, LoaderCircle, Send, X, ClipboardPaste, ChevronDown, History, MessageSquareLock, MessageSquareShare, Globe, ChevronLeft, ChevronRight, ArrowLeft, ExternalLink, Pencil, Check } from "@lucide/vue";
import { isDark } from "../composables/useTheme";
import { useI18n } from "vue-i18n";
import TranslateToolbar from "../components/TranslateToolbar.vue";
const { t } = useI18n();

const router = useRouter();

const inputText = ref("");
const translatedText = ref("");
const isLoading = ref(false);
const abortController = ref<AbortController | null>(null);
const errorMessage = ref("");
type WebSearchStatus = "idle" | "searching" | "error" | "done-searched" | "done-nosearch";
const webSearchStatus = ref<WebSearchStatus>("idle");
const webSearchErrorText = ref("");
const lastResultSearched = ref(false);
const lastResultSources = ref<SearchHit[]>([]);
const sourcesView = ref(false);
const textareaRef = ref<HTMLTextAreaElement | null>(null);
const hasResult = ref(false);
const isRestoringHistory = ref(false);
const growAbove = ref(false);
const pinned = ref(false);
const isEditing = ref(false);
const editedText = ref("");
function togglePin() {
  pinned.value = !pinned.value;
  invoke("set_main_pinned", { pinned: pinned.value });
}
const chevronTransform = (open: boolean) =>
  `rotate(${open === growAbove.value ? 0 : 180}deg)`;
const contentWrapRef = ref<HTMLDivElement | null>(null);
const bodyHeight = ref(0);
let lastSentHeight = 0;
let resizeObserver: ResizeObserver | null = null;
let unlistenConfig: (() => void) | null = null;
let unlistenResume: (() => void) | null = null;

// ── History browsing (terminal-style ↑↓) ──
const historyIndex = ref<number | null>(null);
let draftSnapshot: { input: string; output: string } | null = null;

const activeModelName = computed(() => {
  const m = getActiveModel();
  if (!m) return null;
  return m.model || null;
});

// Resolve the active provider's brand icon (same source as dropdown items)
const activeModelIcon = computed(() => {
  const mode = appConfig.active_mode || "translate";
  const pi = (appConfig as any)[`${mode}_active_provider_index`] ?? 0;
  const prov = appConfig.providers[pi];
  return prov ? getProviderIcon(prov, floatingPresets.value) : "";
});

const activeModelCapabilities = computed<ModelInputCapabilities | undefined>(() => {
  const mode = appConfig.active_mode || "translate";
  const pi = (appConfig as any)[`${mode}_active_provider_index`] ?? 0;
  const mi = (appConfig as any)[`${mode}_active_model_index`] ?? 0;
  const prov = appConfig.providers[pi];
  return prov?.models?.[mi]?.input_capabilities;
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

// When a sparkle with a description is active, surface it as the input placeholder.
const inputPlaceholder = computed(() => {
  if (appConfig.active_mode === "sparkle") {
    const s = sparkleStore.sparkles.find((sp) => sp.enabled);
    const desc = s?.description?.trim();
    if (desc) return desc;
  }
  return hasResult.value ? t("floating.pressEnterToPaste") : t("floating.typeToSend");
});

/** Extract a display hostname from a URL, stripping the leading "www.".
 *  Falls back to the raw string on parse failure. */
function domainOf(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}

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
  handleResultStale();
  flushConfigSave();
}

// Flatten all provider models for dropdown: [{pIndex, mIndex, id, icon, input_capabilities}]
const allModels = computed(() => {
  const result: Array<{ pIndex: number; mIndex: number; id: string; icon: string; input_capabilities?: ModelInputCapabilities }> = [];
  appConfig.providers.forEach((prov, pi) => {
    prov.models.forEach((m, mi) => {
      result.push({
        pIndex: pi,
        mIndex: mi,
        id: m.id,
        icon: getProviderIcon(prov, floatingPresets.value),
        input_capabilities: m.input_capabilities,
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
  handleResultStale();
  if (modeBtnRef.value) {
    burstParticles(modeBtnRef.value);
    popElement(modeBtnRef.value);
  }
}

// Cycle to the next mode (translate → sparkle → … → back to first).
// Reuses selectMode so the same burst/pop animation plays as a click.
function cycleMode() {
  if (MODES.length < 2) return;
  const idx = MODES.findIndex((m) => m.id === appConfig.active_mode);
  const next = MODES[(idx + 1) % MODES.length];
  selectMode(next.id);
}

// Webview-scoped keydown listener: fires the mode-cycle shortcut only while
// FloatingInput is mounted (other views are unmounted, so the listener is gone).
function onModeShortcutKeydown(e: KeyboardEvent) {
  if (eventMatchesShortcut(e, appConfig.mode_shortcut)) {
    e.preventDefault();
    cycleMode();
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

  // 在编辑模式下禁用 Enter 键发送功能
  if (isEditing.value) {
    return;
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
  lastResultSearched.value = !!entry.searched;
  lastResultSources.value = entry.sources ?? [];
  sourcesView.value = false;
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
  webSearchErrorText.value = "";
  isLoading.value = true;
  const controller = new AbortController();
  abortController.value = controller;
  // Reflect search intent up front so the toolbar dot can pulse
  if (appConfig.active_mode === "sparkle" && appConfig.web_search_enabled_in_sparkle) {
    webSearchStatus.value = "searching";
  } else {
    webSearchStatus.value = "idle";
  }

  try {
    const outcome: TranslateOutcome = await translate(text, controller.signal);
    if (outcome.status === "ok") {
      translatedText.value = outcome.content;
      lastResultSearched.value = outcome.searched;
      lastResultSources.value = outcome.sources ?? [];
      sourcesView.value = false;
      webSearchStatus.value = outcome.searched ? "done-searched" : "done-nosearch";
      hasResult.value = true;
      await saveHistoryEntry(text, outcome.content, outcome.searched, outcome.sources);
    }
    // search-error is handled in catch below via SearchFailureError
  } catch (err) {
    if (err instanceof DOMException && err.name === "AbortError") {
      return;
    }
    if (err instanceof SearchFailureError) {
      const classified = classifySearchError(err.cause);
      webSearchStatus.value = "error";
      webSearchErrorText.value =
        t("search.failed", { code: classified.code, message: classified.rawMessage ?? "" }) +
        " " +
        t("search.retryOrDisable");
    } else if (err instanceof ModelHttpError) {
      errorMessage.value = t("failed", { code: err.status, message: err.message });
    } else {
      errorMessage.value = String(err);
    }
  } finally {
    abortController.value = null;
    isLoading.value = false;
  }
}

async function handlePasteResult() {
  const text = translatedText.value;
  if (!text) return;

  if (pinned.value) {
    // Pinned: single atomic command hides → pastes → restores the window, so
    // there's only one IPC round-trip and the hidden window period is minimized.
    await invoke("paste_pinned", { text });
    clearAll();
  } else {
    await invoke("hide_main_window");
    await invoke("simulate_paste", { text });
    clearAll();
  }
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
  if (target.closest("textarea, button, input, a, .model-dropdown, .resize-handle")) return;
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
  webSearchErrorText.value = "";
  webSearchStatus.value = "idle";
  lastResultSources.value = [];
  sourcesView.value = false;
  hasResult.value = false;
}

function closeResult() {
  hasResult.value = false;
  translatedText.value = "";
  lastResultSources.value = [];
  sourcesView.value = false;
}

function startEditing() {
  editedText.value = translatedText.value;
  isEditing.value = true;
}

function cancelEditing() {
  isEditing.value = false;
  editedText.value = "";
}

async function confirmEditing() {
  // 保存到历史记录，添加 "已编辑" tag
  await saveHistoryEntry(inputText.value, editedText.value, lastResultSearched.value, lastResultSources.value, true);
  
  // 更新当前显示
  translatedText.value = editedText.value;
  isEditing.value = false;
  editedText.value = "";
}

function handleEditKeydown(e: KeyboardEvent) {
  // 阻止 Enter 键发送
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
  }
}

function startResize(e: MouseEvent) {
  e.preventDefault();
  e.stopPropagation();
  
  const textarea = (e.target as HTMLElement).closest('.result-edit-textarea-wrapper')?.querySelector('textarea') as HTMLTextAreaElement | null;
  if (!textarea) return;
  
  const startY = e.clientY;
  const startHeight = textarea.offsetHeight;
  
  function onMouseMove(e: MouseEvent) {
    const deltaY = e.clientY - startY;
    const newHeight = Math.max(60, Math.min(200, startHeight + deltaY));
    textarea!.style.height = `${newHeight}px`;
  }
  
  function onMouseUp() {
    document.removeEventListener('mousemove', onMouseMove);
    document.removeEventListener('mouseup', onMouseUp);
  }
  
  document.addEventListener('mousemove', onMouseMove);
  document.addEventListener('mouseup', onMouseUp);
}

function cancelRequest() {
  abortController.value?.abort();
  abortController.value = null;
  isLoading.value = false;
  webSearchStatus.value = "idle";
  errorMessage.value = "";
  webSearchErrorText.value = "";
}

onMounted(async () => {
  lastSentHeight = 0;

  // Config is loaded once at startup (main.ts) and shared as a single reactive
  // instance across all views — do not reload here, or disk (possibly stale)
  // would overwrite in-memory edits made in other views.
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
      lastResultSearched.value = !!entry.searched;
      lastResultSources.value = entry.sources ?? [];
      sourcesView.value = false;
      nextTick(() => { isRestoringHistory.value = false; });
    } catch { /* ignore */ }
  }
  document.addEventListener("mousedown", onDocumentClick);
  window.addEventListener("keydown", onModeShortcutKeydown);
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

  // After system resume (lid close/open), DWM hide/show fixes the composited
  // surface but Vue layout can go stale. Force a reflow by re-sending the
  // current height to the backend.
  unlistenResume = await listen("system-resumed", () => {
    invoke("resize_and_reposition", { height: bodyHeight.value, width: MAIN_WIDTH });
  });
});

// Config auto-save is centralized in stores/config.ts (enabled at startup).

onUnmounted(() => {
  document.removeEventListener("mousedown", onDocumentClick);
  window.removeEventListener("keydown", onModeShortcutKeydown);
  unlistenConfig?.();
  unlistenResume?.();
  resizeObserver?.disconnect();
});

// Resize window when content changes
watch(bodyHeight, (h) => {
  if (h !== lastSentHeight) {
    lastSentHeight = h;
    invoke("resize_and_reposition", { height: h, width: MAIN_WIDTH });
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
    class="w-full h-[100dvh] flex justify-center overflow-hidden"
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
            <!-- Edit button (top-left corner) - replaces close button position -->
            <button
              v-if="!isEditing"
              @click="startEditing"
              class="edit-result-btn"
              :title="t('common.edit')"
            >
              <Pencil :size="12" />
            </button>
            
            <!-- Close result button (top-right corner) - moved -->
            <button
              @click="closeResult"
              class="close-result-btn"
              :title="t('common.close')"
              :class="{ 'editing-mode': isEditing }"
            >
              <X :size="12" />
            </button>
            
            <div v-if="lastResultSearched && !isEditing" class="provenance-row">
              <button class="provenance-btn" @click="sourcesView = !sourcesView">
                <Globe :size="11" :stroke-width="1.8" />
                <span>{{ t('search.sourceSearched') }}</span>
                <component :is="sourcesView ? ChevronLeft : ChevronRight" :size="11" :stroke-width="2" class="provenance-chevron" />
              </button>
              <button v-if="sourcesView" class="sources-back" @click="sourcesView = false">
                <ArrowLeft :size="12" :stroke-width="1.8" /> {{ t('search.backToResult') }}
              </button>
            </div>
            
            <!-- Result view (default) -->
            <div v-if="!isEditing" v-show="!sourcesView" class="result-text">{{ translatedText }}</div>
            
            <!-- Edit mode -->
            <div v-if="isEditing" class="result-edit-textarea-wrapper">
              <textarea
                v-model="editedText"
                class="result-edit-textarea"
                @keydown="handleEditKeydown"
              ></textarea>
              <div class="resize-handle" @mousedown="startResize">
                <svg width="14" height="14" viewBox="0 0 18 18" xmlns="http://www.w3.org/2000/svg">
                  <path fill="currentColor" d="M14.228 16.227a1 1 0 0 1-.707-1.707l1-1a1 1 0 0 1 1.416 1.414l-1 1a1 1 0 0 1-.707.293zm-5.638 0a1 1 0 0 1-.707-1.707l6.638-6.638a1 1 0 0 1 1.416 1.414l-6.638 6.638a1 1 0 0 1-.707.293z"/>
                </svg>
              </div>
            </div>
            
            <!-- Edit action buttons (bottom-right) -->
            <div v-if="isEditing" class="edit-actions">
              <button @click="cancelEditing" class="edit-action-btn cancel" :title="t('common.cancel')">
                <X :size="14" />
              </button>
              <button @click="confirmEditing" class="edit-action-btn confirm" :title="t('common.confirm')">
                <Check :size="14" />
              </button>
            </div>
            
            <!-- Sources view (shown when the provenance button is toggled on) -->
            <div v-if="sourcesView && !isEditing" class="sources-view">
              <a v-for="(src, i) in lastResultSources" :key="i"
                 :href="src.url" target="_blank" rel="noopener noreferrer" class="source-item">
                <div class="source-favicon">🌐</div>
                <div class="source-meta">
                  <div class="source-title">{{ src.title || t('search.untitledSource') }}</div>
                  <div class="source-domain">{{ domainOf(src.url) }}</div>
                </div>
                <ExternalLink :size="11" :stroke-width="1.8" class="source-external" />
              </a>
              <div v-if="lastResultSources.length === 0" class="sources-empty">{{ t('search.noSources') }}</div>
            </div>
          </div>
        </Transition>

        <!-- AI disclaimer (appears only when a result is present) -->
        <Transition name="fade">
          <div v-show="translatedText" class="ai-disclaimer">
            {{ t('floating.aiDisclaimer') }}
          </div>
        </Transition>

        <!-- Loading state -->
        <Transition name="fade">
          <div
            v-show="isLoading"
            class="flex items-center gap-2 text-[11px] text-[var(--color-text-secondary)]"
          >
            <span class="inline-block w-1.5 h-1.5 rounded-full bg-amber-400/60 animate-pulse"></span>
            {{ webSearchStatus === 'searching' ? t('floating.searching') : t('floating.sending') }}
            <button
              @click="cancelRequest"
              class="cancel-request-btn"
              :title="t('common.cancel')"
            >
              <X :size="12" />
            </button>
          </div>
        </Transition>

        <!-- Web search error (blocking; one line, no buttons) -->
        <Transition name="fade">
          <div
            v-show="webSearchStatus === 'error' && webSearchErrorText"
            class="text-[11px] text-red-400/80"
          >
            {{ webSearchErrorText }}
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
              :placeholder="inputPlaceholder"
              rows="1"
              class="floating-input w-full resize-none text-[13px] leading-relaxed outline-none"
            ></textarea>

            <!-- History button (top-left corner of textarea) -->
            <button
              v-if="!isEditing"
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
            :disabled="(!inputText.trim() && !hasResult) || isLoading || isEditing"
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
        <div class="flex items-center gap-2" :class="{ 'toolbar-disabled': isEditing }">
          <!-- Mode switch button -->
          <div class="relative">
            <button
              ref="modeBtnRef"
              @click="toggleModeDropdown"
              class="mode-btn"
              :class="{ active: showModeDropdown }"
              :title="t(currentMode.labelKey)"
              :disabled="isEditing"
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
              <span class="model-icon" v-if="activeModelIcon"><ProviderIcon :icon="activeModelIcon" :size="14" /></span>
              <span class="truncate max-w-[120px] min-w-0">{{ activeModelName }}</span>
              <ModelCapabilityIcon :capabilities="activeModelCapabilities" :size="10" />
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
                    <div class="opt-left"><ProviderIcon :icon="entry.icon" :size="14" />
                    <span class="truncate">{{ entry.id }}</span>
                    <ModelCapabilityIcon :capabilities="entry.input_capabilities" :size="11" /></div>
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

          <div class="flex items-center gap-1">
            <button
              @click="togglePin"
              class="icon-btn"
              :class="{ 'pin-active': pinned }"
              :title="pinned ? t('common.keepOpenAfterSend') : t('common.closeAfterSend')"
            >
              <MessageSquareLock v-if="pinned" :size="14" :stroke-width="1.8" />
              <MessageSquareShare v-else :size="14" :stroke-width="1.8" />
            </button>

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
        </div>
      </template>

      <!-- !growAbove: result grows downward, input at top (default) -->
      <template v-else>
        <!-- Toolbar -->
        <div class="flex items-center gap-2" :class="{ 'toolbar-disabled': isEditing }">
          <!-- Mode switch button -->
          <div class="relative">
            <button
              ref="modeBtnRef"
              @click="toggleModeDropdown"
              class="mode-btn"
              :class="{ active: showModeDropdown }"
              :title="t(currentMode.labelKey)"
              :disabled="isEditing"
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
              <span class="model-icon" v-if="activeModelIcon"><ProviderIcon :icon="activeModelIcon" :size="14" /></span>
              <span class="truncate max-w-[120px] min-w-0">{{ activeModelName }}</span>
              <ModelCapabilityIcon :capabilities="activeModelCapabilities" :size="10" />
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
                    <div class="opt-left"><ProviderIcon :icon="entry.icon" :size="14" />
                    <span class="truncate">{{ entry.id }}</span>
                    <ModelCapabilityIcon :capabilities="entry.input_capabilities" :size="11" /></div>
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

          <div class="flex items-center gap-1">
            <button
              @click="togglePin"
              class="icon-btn"
              :class="{ 'pin-active': pinned }"
              :title="pinned ? t('common.keepOpenAfterSend') : t('common.closeAfterSend')"
            >
              <MessageSquareLock v-if="pinned" :size="14" :stroke-width="1.8" />
              <MessageSquareShare v-else :size="14" :stroke-width="1.8" />
            </button>

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
        </div>

        <!-- Input area + inline send -->
        <div class="relative">
          <div class="textarea-with-history">
            <textarea
              ref="textareaRef"
              v-model="inputText"
              @keydown="handleKeydown"
              :placeholder="inputPlaceholder"
              rows="1"
              class="floating-input w-full resize-none text-[13px] leading-relaxed outline-none"
            ></textarea>

            <!-- History button (top-left corner of textarea) -->
            <button
              v-if="!isEditing"
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
            :disabled="(!inputText.trim() && !hasResult) || isLoading || isEditing"
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
            {{ webSearchStatus === 'searching' ? t('floating.searching') : t('floating.sending') }}
            <button
              @click="cancelRequest"
              class="cancel-request-btn"
              :title="t('common.cancel')"
            >
              <X :size="12" />
            </button>
          </div>
        </Transition>

        <!-- Web search error (blocking; one line, no buttons) -->
        <Transition name="fade">
          <div
            v-show="webSearchStatus === 'error' && webSearchErrorText"
            class="text-[11px] text-red-400/80"
          >
            {{ webSearchErrorText }}
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
            <!-- Edit button (top-left corner) - replaces close button position -->
            <button
              v-if="!isEditing"
              @click="startEditing"
              class="edit-result-btn"
              :title="t('common.edit')"
            >
              <Pencil :size="12" />
            </button>
            
            <!-- Close result button (top-right corner) - moved -->
            <button
              @click="closeResult"
              class="close-result-btn"
              :title="t('common.close')"
              :class="{ 'editing-mode': isEditing }"
            >
              <X :size="12" />
            </button>
            
            <div v-if="lastResultSearched && !isEditing" class="provenance-row">
              <button class="provenance-btn" @click="sourcesView = !sourcesView">
                <Globe :size="11" :stroke-width="1.8" />
                <span>{{ t('search.sourceSearched') }}</span>
                <component :is="sourcesView ? ChevronLeft : ChevronRight" :size="11" :stroke-width="2" class="provenance-chevron" />
              </button>
              <button v-if="sourcesView" class="sources-back" @click="sourcesView = false">
                <ArrowLeft :size="12" :stroke-width="1.8" /> {{ t('search.backToResult') }}
              </button>
            </div>
            
            <!-- Result view (default) -->
            <div v-if="!isEditing" v-show="!sourcesView" class="result-text">{{ translatedText }}</div>
            
            <!-- Edit mode -->
            <div v-if="isEditing" class="result-edit-textarea-wrapper">
              <textarea
                v-model="editedText"
                class="result-edit-textarea"
                @keydown="handleEditKeydown"
              ></textarea>
              <div class="resize-handle" @mousedown="startResize">
                <svg width="14" height="14" viewBox="0 0 18 18" xmlns="http://www.w3.org/2000/svg">
                  <path fill="currentColor" d="M14.228 16.227a1 1 0 0 1-.707-1.707l1-1a1 1 0 0 1 1.416 1.414l-1 1a1 1 0 0 1-.707.293zm-5.638 0a1 1 0 0 1-.707-1.707l6.638-6.638a1 1 0 0 1 1.416 1.414l-6.638 6.638a1 1 0 0 1-.707.293z"/>
                </svg>
              </div>
            </div>
            
            <!-- Edit action buttons (bottom-right) -->
            <div v-if="isEditing" class="edit-actions">
              <button @click="cancelEditing" class="edit-action-btn cancel" :title="t('common.cancel')">
                <X :size="14" />
              </button>
              <button @click="confirmEditing" class="edit-action-btn confirm" :title="t('common.confirm')">
                <Check :size="14" />
              </button>
            </div>
            
            <!-- Sources view (shown when the provenance button is toggled on) -->
            <div v-if="sourcesView && !isEditing" class="sources-view">
              <a v-for="(src, i) in lastResultSources" :key="i"
                 :href="src.url" target="_blank" rel="noopener noreferrer" class="source-item">
                <div class="source-favicon">🌐</div>
                <div class="source-meta">
                  <div class="source-title">{{ src.title || t('search.untitledSource') }}</div>
                  <div class="source-domain">{{ domainOf(src.url) }}</div>
                </div>
                <ExternalLink :size="11" :stroke-width="1.8" class="source-external" />
              </a>
              <div v-if="lastResultSources.length === 0" class="sources-empty">{{ t('search.noSources') }}</div>
            </div>
          </div>
        </Transition>

        <!-- AI disclaimer (appears only when a result is present) -->
        <Transition name="fade">
          <div v-show="translatedText" class="ai-disclaimer">
            {{ t('floating.aiDisclaimer') }}
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

/* Cancel request button */
.cancel-request-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border-radius: 4px;
  color: var(--color-text-secondary);
  background: transparent;
  border: none;
  cursor: pointer;
  transition: all 0.15s ease;
  margin-left: 4px;
}

.cancel-request-btn:hover {
  color: var(--color-danger);
  background: var(--color-danger-bg);
}

.cancel-request-btn:active {
  transform: scale(0.9);
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

/* Brand icon: keep it full-size and outside the truncated text area */
.model-icon {
  display: inline-flex;
  align-items: center;
  flex-shrink: 0;
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
  justify-content: flex-start;
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

.opt-left { display:flex; align-items:center; gap:6px; min-width:0; flex:1; }
.check-mark {
  font-size: 10px;
  flex-shrink: 0;
  margin-left: auto;
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

/* Pin button — accent-colored when pinned, to signal the window stays open */
.icon-btn.pin-active {
  color: var(--color-accent);
  background: color-mix(in srgb, var(--color-accent) 12%, var(--color-surface));
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
  overflow: visible;
  position: relative;
}

/* Edit button - replaces close button position */
.edit-result-btn {
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
  background: var(--color-bg) !important;
}

.result-block:hover .edit-result-btn {
  opacity: 1 !important;
}

.edit-result-btn:hover {
  color: var(--color-accent) !important;
  border-color: var(--color-accent) !important;
  background: color-mix(in srgb, var(--color-accent) 12%, var(--color-bg)) !important;
}

/* Close result button - moved to top-right */
.close-result-btn {
  position: absolute !important;
  top: -11px !important;
  right: -11px !important;
  left: auto !important;
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
  background: var(--color-bg) !important;
}

.result-block:hover .close-result-btn {
  opacity: 1 !important;
}

.close-result-btn:hover {
  color: var(--color-danger) !important;
  border-color: var(--color-danger) !important;
  background: color-mix(in srgb, var(--color-danger) 12%, var(--color-bg)) !important;
}

/* Edit textarea */
.result-edit-textarea {
  width: 100%;
  padding: 12px 14px;
  font-size: 13px;
  line-height: 1.65;
  color: var(--color-text);
  background: transparent;
  border: none;
  outline: none;
  resize: none;
  min-height: 60px;
  max-height: 200px;
  overflow-y: auto;
  position: relative;
  z-index: 1;
  box-sizing: border-box;
}

.result-edit-textarea-wrapper {
  position: relative;
}

.resize-handle {
  position: absolute;
  bottom: 2px;
  right: 2px;
  width: 12px;
  height: 12px;
  cursor: ns-resize;
  opacity: 0.3;
  transition: opacity 0.15s;
  z-index: 2;
}

.resize-handle:hover {
  opacity: 0.6;
}

.resize-handle svg {
  width: 100%;
  height: 100%;
}

/* Edit action buttons */
.edit-actions {
  position: absolute;
  bottom: 8px;
  right: 8px;
  display: flex;
  gap: 4px;
  z-index: 2;
}

.edit-action-btn {
  width: 24px;
  height: 24px;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
  background: transparent;
  border: none;
  color: var(--color-text-muted);
  cursor: pointer;
}

.edit-action-btn:hover {
  background: var(--color-surface-hover);
  color: var(--color-text);
}

.edit-action-btn.confirm:hover {
  color: var(--color-accent);
  background: var(--color-accent-bg);
}

.edit-action-btn.cancel:hover {
  color: var(--color-danger);
  background: var(--color-danger-bg);
}

/* Disable toolbar buttons in edit mode */
.toolbar-disabled {
  opacity: 0.5;
  pointer-events: none;
}

.result-text {
  padding: 12px 14px;
  font-size: 13px;
  line-height: 1.65;
  color: var(--color-text);
  white-space: pre-wrap;
  word-break: break-word;
  overflow-y: auto;
  max-height: 200px;
  position: relative;
  z-index: 1;
}

/* AI disclaimer shown below the result */
.ai-disclaimer {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 0 12px;
  font-size: 10.5px;
  line-height: 1.4;
  color: var(--color-text-secondary);
  opacity: 0.7;
  flex-shrink: 0;
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
  .icon-btn.active { animation: none;
}
}

/* ── Web search provenance button + sources view ── */
.provenance-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 0 5px 0;
}
.provenance-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 10.5px;
  color: var(--color-text-secondary);
  background: none;
  border: none;
  cursor: pointer;
  padding: 0;
  transition: color 0.12s;
}
.provenance-btn:hover { color: var(--color-accent); }
.provenance-chevron { margin-left: 1px; }
.sources-back {
  display: inline-flex; align-items: center; gap: 4px;
  font-size: 10.5px; color: var(--color-text-muted);
  background: none; border: none; cursor: pointer; padding: 0;
  transition: color 0.12s;
}
.sources-back:hover { color: var(--color-text); }

.sources-view {
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-height: 200px;
  overflow-y: auto;
  padding-bottom: 4px;
}
.sources-view::-webkit-scrollbar { width: 3px; }
.sources-view::-webkit-scrollbar-thumb { background: var(--color-scrollbar); border-radius: 3px; }
.source-item {
  display: flex; align-items: center; gap: 8px;
  padding: 7px 9px; border-radius: 8px;
  border: 1px solid var(--color-border);
  background: var(--color-surface);
  text-decoration: none; cursor: pointer;
  transition: background 0.12s, border-color 0.12s;
}
.source-item:hover { background: var(--color-surface-hover); border-color: var(--color-border-hover); }
.source-favicon { flex-shrink: 0; font-size: 13px; line-height: 1; }
.source-meta { display: flex; flex-direction: column; gap: 1px; min-width: 0; flex: 1; }
.source-title {
  font-size: 11.5px; font-weight: 600; color: var(--color-text);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.source-domain {
  font-size: 10px; color: var(--color-text-muted);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.source-external { flex-shrink: 0; color: var(--color-text-muted); }
.sources-empty { font-size: 10.5px; color: var(--color-text-muted); padding: 8px 0; }
</style>
