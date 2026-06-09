<script setup lang="ts">
import { ref, reactive, computed, watch, onMounted, onUnmounted, nextTick } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useRouter, useRoute } from "vue-router";
import {
  appConfig,
  personaStore,
  loadConfig,
  saveConfig as persistConfig,
  savePersonas as persistPersonas,
  getOrderedLanguages,
  loadProviderPresets,
  dictStore,
  refreshDictStatus,
  clearAllHistory,
} from "../stores/config";
import type { ProviderConfig, ProviderPreset } from "../stores/config";
import { getTheme, setTheme } from "../composables/useTheme";
import { useSettingsWindow } from "../composables/useSettingsWindow";
import { testProviderConnection, fetchProviderModels, optimizePrompt } from "../services/llm-client";
import { BUILTIN_LANGUAGES, getLangName } from "../constants/languages";
import draggable from "vuedraggable";
import EditableCardList from "../components/EditableCardList.vue";
import {
  ArrowLeft,
  Languages,
  UserCircle,
  Settings2,
  Plus,
  Trash2,
  Check,
  Eye,
  EyeOff,
  Link2,
  Loader2,
  RefreshCw,
  ChevronDown,
  Pencil,
  Cpu,
  CircleDot,
  X,
  BookText,
  GripVertical,
  RotateCcw,
  Wand2,
  Sun,
  Moon,
  SunMoon,
  Info,
  ToggleLeft,
  ToggleRight,
  Droplet,
  Database,
  Monitor,
  History,
} from "@lucide/vue";

declare const __APP_VERSION__: string;
const appVersion = __APP_VERSION__;

const { t } = useI18n();

type TabKey = "general" | "translation";

const router = useRouter();
const route = useRoute();
const { growAbove } = useSettingsWindow();
const activeTab = ref<TabKey>("general");
const testingProvider = ref<number | null>(null);
const optimizingIndex = ref<number | null>(null);
const promptUndoStack = new Map<number, string>();

interface ProviderEditState {
  keyVisible: boolean;
  fetching: boolean;
  fetched: string[];
  status: string;
}
const editStates = ref<Map<number, ProviderEditState>>(new Map());

function getEditState(index: number): ProviderEditState {
  let s = editStates.value.get(index);
  if (!s) {
    s = reactive({ keyVisible: false, fetching: false, fetched: [], status: "" });
    editStates.value.set(index, s);
    editStates.value = new Map(editStates.value);
  }
  return s;
}

function clearEditState(index: number) {
  editStates.value.delete(index);
}

const addingModelProvider = ref<number | null>(null);
const showModelSelector = ref(false);
const showLangSelector = ref(false);
const showPresetMenu = ref(false);
const presetMenuPos = ref({ top: 0, left: 0 });
const presetMenuIndex = ref<number | null>(null);
const providerPresets = ref<ProviderPreset[]>([]);
const selMenuPos = ref({ top: 0, left: 0 });
const langMenuPos = ref({ top: 0, left: 0 });
const selBtnRef = ref<HTMLElement | null>(null);
const langBtnRef = ref<HTMLElement | null>(null);

// ── App language switcher ──
const appLanguageOptions = [
  { value: "en", label: "English" },
  { value: "zh-CN", label: "简体中文" },
];

const showAppLangMenu = ref(false);
const showHistoryClearConfirm = ref(false);
const appLangMenuPos = ref({ top: 0, left: 0, width: 0 });
const appLangBtnRef = ref<HTMLElement | null>(null);

function toggleAppLangMenu() {
  showAppLangMenu.value = !showAppLangMenu.value;
  if (showAppLangMenu.value && appLangBtnRef.value) {
    const r = appLangBtnRef.value.getBoundingClientRect();
    appLangMenuPos.value = { top: r.bottom + 5, left: r.left, width: r.width };
  }
}

function selectAppLang(lang: string) {
  appConfig.app_lang = lang;
  showAppLangMenu.value = false;
}

const currentAppLangLabel = computed(() => {
  return appLanguageOptions.find(o => o.value === appConfig.app_lang)?.label || "English";
});

// ── Auto-update ──
// idle | checking | up-to-date | has-update | preparing | downloading | installing | restarting | error
const updateStatus = ref("idle");
const updateVersion = ref("");
const downloaded = ref(0);
const contentLength = ref(0);
const updateError = ref("");
const autoUpdate = ref(localStorage.getItem("app-auto-update") !== "false");
const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

function toggleAutoUpdate() {
  autoUpdate.value = !autoUpdate.value;
  localStorage.setItem("app-auto-update", String(autoUpdate.value));
}

async function checkForUpdate(silent = false) {
  if (!isTauri) return;
  updateStatus.value = "checking";
  updateError.value = "";
  try {
    const { check } = await import("@tauri-apps/plugin-updater");
    const proxy = await invoke<string | null>("get_proxy_url");
    const update = await check(proxy ? { proxy } : {});
    if (!update) {
      if (!silent) {
        updateStatus.value = "up-to-date";
        setTimeout(() => { updateStatus.value = "idle"; }, 2000);
      } else {
        updateStatus.value = "idle";
      }
      return;
    }
    updateVersion.value = update.version;
    updateStatus.value = "has-update";
  } catch (e) {
    if (!silent) {
      updateStatus.value = "error";
      updateError.value = e instanceof Error ? e.message : String(e);
      setTimeout(() => { updateStatus.value = "idle"; updateError.value = ""; }, 3000);
    } else {
      updateStatus.value = "idle";
    }
  }
}

async function installUpdate() {
  if (!isTauri) return;
  try {
    const { check } = await import("@tauri-apps/plugin-updater");
    const { relaunch } = await import("@tauri-apps/plugin-process");
    const proxy = await invoke<string | null>("get_proxy_url");
    const update = await check(proxy ? { proxy } : {});
    if (!update) return;
    updateStatus.value = "preparing";
    downloaded.value = 0;
    contentLength.value = 0;
    await update.downloadAndInstall((event) => {
      switch (event.event) {
        case "Started":
          contentLength.value = event.data.contentLength || 0;
          updateStatus.value = "downloading";
          break;
        case "Progress":
          downloaded.value += event.data.chunkLength;
          break;
        case "Finished":
          updateStatus.value = "installing";
          break;
      }
    });
    updateStatus.value = "restarting";
    await relaunch();
  } catch (e) {
    updateStatus.value = "error";
    updateError.value = e instanceof Error ? e.message : String(e);
    setTimeout(() => { updateStatus.value = "idle"; updateError.value = ""; }, 3000);
  }
}

function handleUpdateClick() {
  if (updateStatus.value === "has-update") installUpdate();
  else if (["idle", "up-to-date", "error"].includes(updateStatus.value)) checkForUpdate(false);
}

// ── Persona management ──
function validateProvider(p: ProviderConfig): string | null {
  const missing: string[] = [];
  if (!p.name.trim()) missing.push("Name");
  if (!p.api_key.trim()) missing.push("API Key");
  if (!p.base_url.trim()) missing.push("Base URL");
  if (p.models.length === 0) missing.push("at least one Model");
  return missing.length ? `Required: ${missing.join(", ")}` : null;
}

function validatePersona(p: { name: string; prompt: string }): string | null {
  const missing: string[] = [];
  if (!p.name.trim()) missing.push("Name");
  if (!p.prompt.trim()) missing.push("Prompt");
  return missing.length ? `Required: ${missing.join(", ")}` : null;
}

function togglePersona(index: number) {
  const wasOn = personaStore.personas[index].enabled;
  for (const p of personaStore.personas) p.enabled = false;
  if (!wasOn) personaStore.personas[index].enabled = true;
}

async function handleOptimizePrompt(item: { prompt: string }, index: number) {
  if (!item.prompt.trim() || optimizingIndex.value !== null) return;
  promptUndoStack.set(index, item.prompt);
  optimizingIndex.value = index;
  try {
    item.prompt = await optimizePrompt(item.prompt);
  } catch (err) {
    console.error("Optimize failed:", err);
    promptUndoStack.delete(index);
  } finally {
    optimizingIndex.value = null;
  }
}

function handleTextareaKeydown(e: KeyboardEvent, item: { prompt: string }, index: number) {
  const isMod = e.ctrlKey || e.metaKey;
  if (isMod && e.key === "z" && !e.shiftKey && promptUndoStack.has(index)) {
    e.preventDefault();
    item.prompt = promptUndoStack.get(index)!;
    promptUndoStack.delete(index);
  }
}

function toggleSelMenu() {
  if (allFlat.value.length === 0) return;
  showLangSelector.value = false;
  showModelSelector.value = !showModelSelector.value;
  if (showModelSelector.value && selBtnRef.value) {
    const r = selBtnRef.value.getBoundingClientRect();
    selMenuPos.value = { top: r.bottom + 5, left: r.left };
  }
}

function toggleLangMenu() {
  showModelSelector.value = false;
  showLangSelector.value = !showLangSelector.value;
  if (showLangSelector.value && langBtnRef.value) {
    const r = langBtnRef.value.getBoundingClientRect();
    langMenuPos.value = { top: r.bottom + 5, left: r.left };
  }
}

function pickLang(lang: string) {
  appConfig.target_lang = lang;
  showLangSelector.value = false;
}

function togglePresetMenu(e: MouseEvent, _item: ProviderConfig, index: number) {
  showModelSelector.value = false;
  showLangSelector.value = false;
  if (showPresetMenu.value && presetMenuIndex.value === index) {
    showPresetMenu.value = false;
    presetMenuIndex.value = null;
    return;
  }
  presetMenuIndex.value = index;
  showPresetMenu.value = true;
  const btn = e.currentTarget as HTMLElement;
  const r = btn.getBoundingClientRect();
  presetMenuPos.value = { top: r.bottom + 5, left: r.right - 220 };
}

function applyPreset(item: ProviderConfig, preset: ProviderPreset) {
  item.preset = preset.name !== "Custom" ? preset.name : undefined;
  item.base_url = preset.base_url;
  item.api_format = preset.api_format && Object.keys(preset.api_format).length > 0 ? { ...preset.api_format } : undefined;
  if (!item.name.trim()) item.name = preset.provider_name;
  showPresetMenu.value = false;
  presetMenuIndex.value = null;
}

// ── Language management ──
const newLangInput = ref("");
const showAddLang = ref(false);
const langAddInputRef = ref<HTMLInputElement | null>(null);

watch(showAddLang, (val) => {
  if (val) nextTick(() => langAddInputRef.value?.focus());
});

interface LangItem {
  id: string;
  name: string;
  isCustom: boolean;
}

const langItems = computed<LangItem[]>(() => {
  return getOrderedLanguages().map(name => ({
    id: name,
    name,
    isCustom: !BUILTIN_LANGUAGES.includes(name),
  }));
});

function onLangDragEnd() {
  appConfig.language_order = langItems.value.map(item => item.name);
}

function onProviderDragEnd({ indexMap }: { indexMap: Map<number, number> }) {
  appConfig.active_provider_index = indexMap.get(appConfig.active_provider_index) ?? 0;

  const re = new Map<number, ProviderEditState>();
  for (const [k, v] of editStates.value) {
    const m = indexMap.get(k);
    if (m !== undefined) re.set(m, v);
  }
  editStates.value = re;

  if (testingProvider.value !== null) testingProvider.value = indexMap.get(testingProvider.value) ?? null;
  if (addingModelProvider.value !== null) addingModelProvider.value = indexMap.get(addingModelProvider.value) ?? null;
}

function deleteCustomLang(name: string) {
  appConfig.custom_languages = appConfig.custom_languages.filter(l => l !== name);
  appConfig.language_order = appConfig.language_order.filter(l => l !== name);
  if (appConfig.target_lang === name) {
    appConfig.target_lang = "English";
  }
}

function addCustomLang() {
  const name = newLangInput.value.trim();
  if (!name) return;
  const allNames = getOrderedLanguages();
  if (allNames.some(l => l.toLowerCase() === name.toLowerCase())) {
    newLangInput.value = "";
    return;
  }
  appConfig.custom_languages.push(name);
  appConfig.language_order = [...getOrderedLanguages(), name];
  newLangInput.value = "";
  showAddLang.value = false;
}

function restoreDefaultOrder() {
  appConfig.language_order = [];
}

function toggleKeyVisibility(index: number) {
  const s = getEditState(index);
  s.keyVisible = !s.keyVisible;
}

async function load() {
  try { await loadConfig(); }
  catch (err) { console.error("Failed to load config:", err); }
  refreshDictStatus();
}

// ── Auto-save (instant) ──
watch(
  () => JSON.stringify(appConfig),
  () => { persistConfig(); },
);

watch(
  () => JSON.stringify(personaStore.personas),
  () => { persistPersonas(); },
);

function onProviderAdd() {
  appConfig.providers.push({
    name: "",
    api_key: "",
    base_url: "",
    models: [], temperature: 0.3, max_tokens: 1024,
  });
}

function onProviderConfirm({ index }: { index: number }) {
  clearEditState(index);
  persistConfig();
}

function onProviderCancel({ index }: { index: number }) {
  clearEditState(index);
  if (testingProvider.value === index) testingProvider.value = null;
  showPresetMenu.value = false;
  presetMenuIndex.value = null;
}

function onProviderRemove({ index, indexMap }: { index: number; indexMap: Map<number, number> }) {
  clearEditState(index);
  const re = new Map<number, ProviderEditState>();
  for (const [k, v] of editStates.value) {
    const m = indexMap.get(k);
    if (m !== undefined) re.set(m, v);
  }
  editStates.value = re;
  if (appConfig.active_provider_index >= appConfig.providers.length)
    appConfig.active_provider_index = Math.max(0, appConfig.providers.length - 1);
  const ap = appConfig.providers[appConfig.active_provider_index];
  if (ap && appConfig.active_model_index >= ap.models.length)
    appConfig.active_model_index = Math.max(0, ap.models.length - 1);
}

function removeModel(item: ProviderConfig, mIndex: number) {
  item.models.splice(mIndex, 1);
}

async function testConnection(provider: ProviderConfig, index: number) {
  if (!provider.api_key || !provider.base_url) return;
  testingProvider.value = index;
  const s = getEditState(index);
  const result = await testProviderConnection(provider);
  if (result.ok) {
    s.status = "Connected";
    setTimeout(() => { s.status = ""; }, 3000);
  } else {
    s.status = result.error || "Connection failed";
    setTimeout(() => { s.status = ""; }, 4000);
  }
  testingProvider.value = null;
}

async function fetchModels(provider: ProviderConfig, index: number) {
  if (!provider.api_key || !provider.base_url) return;
  const s = getEditState(index);
  s.fetching = true;
  const result = await fetchProviderModels(provider);
  if (result.ok && result.models) {
    s.fetched = result.models;
  } else {
    s.status = result.error || "Fetch failed";
    setTimeout(() => { s.status = ""; }, 5000);
  }
  s.fetching = false;
}

function toggleModel(item: ProviderConfig, mid: string) {
  const idx = item.models.findIndex((m) => m.id === mid);
  if (idx >= 0) {
    item.models.splice(idx, 1);
  } else {
    item.models.push({ id: mid });
  }
}

function getFetchedModels(pi: number): string[] {
  return editStates.value.get(pi)?.fetched || [];
}

// ── Translation model selector ──

interface FlatEntry { pIndex: number; mIndex: number; id: string; providerName: string; }

const allFlat = computed<FlatEntry[]>(() => {
  const out: FlatEntry[] = [];
  appConfig.providers.forEach((prov, pi) =>
    prov.models.forEach((m, mi) =>
      out.push({ pIndex: pi, mIndex: mi, id: m.id, providerName: prov.name || `Provider ${pi + 1}` })
    )
  );
  return out;
});

const activeLabel = computed(() => {
  const { active_provider_index: pi, active_model_index: mi, providers } = appConfig;
  if (pi >= providers.length) return "None";
  const p = providers[pi];
  if (!p || mi >= p.models.length) return "None";
  return p.models[mi].id;
});

function pickModel(e: FlatEntry) {
  appConfig.active_provider_index = e.pIndex;
  appConfig.active_model_index = e.mIndex;
  showModelSelector.value = false;
}

// ── Click outside panels ──

function onDocClick(e: MouseEvent) {
  const t = e.target as HTMLElement;
  if (!t.closest(".sel-menu") && !t.closest(".sel-btn"))
    showModelSelector.value = false;
  if (!t.closest(".sel-menu") && !t.closest(".sel-btn"))
    showAppLangMenu.value = false;
  if (!t.closest(".lang-menu") && !t.closest(".lang-btn"))
    showLangSelector.value = false;
  if (!t.closest(".preset-menu") && !t.closest(".preset-mini-btn")) {
    showPresetMenu.value = false;
    presetMenuIndex.value = null;
  }
  if (!t.closest(".pickable"))
    addingModelProvider.value = null;
}

// ── Navigation ──

async function goBack() {
  router.push("/");
}

async function closeWindow() {
  await invoke("hide_main_window");
}

async function handleDrag(e: MouseEvent) {
  const t = e.target as HTMLElement;
  if (t.closest("textarea, button, input, select, a, .ecl-card, .card-drag-handle, .sel-menu")) return;
  await getCurrentWindow().startDragging();
}

onMounted(async () => {
  if (route.query.tab === "translation") {
    activeTab.value = "translation";
  }
  if (route.query.scrollTo === "persona") {
    nextTick(() => {
      document.getElementById("persona-section")?.scrollIntoView({ behavior: "smooth" });
    });
  }
  document.addEventListener("mousedown", onDocClick);
  load();
  loadProviderPresets().then(p => { providerPresets.value = p; }).catch(console.error);
  if (autoUpdate.value) checkForUpdate(true);
});

onUnmounted(() => {
  document.removeEventListener("mousedown", onDocClick);
});
</script>

<template>
  <div class="settings-root" :class="{ 'grow-above': growAbove }" @mousedown="handleDrag">
    <!-- ═══ Header ═══ -->
    <header class="settings-header">
      <button @click="goBack" class="back-btn" title="Back">
        <ArrowLeft :size="18" :stroke-width="1.8" />
      </button>
      <h1 class="header-title">{{ t('common.settings') }}</h1>
      <button @click="closeWindow" class="close-btn" :title="t('settings.close')">
        <X :size="16" :stroke-width="1.8" />
      </button>
    </header>

    <!-- ═══ Tabs ═══ -->
    <nav class="tabs">
      <button
        v-for="tab in [{ key: 'general' as TabKey, label: t('settings.general'), icon: Settings2 }, { key: 'translation' as TabKey, label: t('settings.translation'), icon: Languages }]"
        :key="tab.key"
        class="tab"
        :class="{ on: activeTab === tab.key }"
        @click="activeTab = tab.key"
      >
        <component :is="tab.icon" :size="13" :stroke-width="1.7" />
        {{ tab.label }}
      </button>
    </nav>

    <!-- ═══ Body ═══ -->
    <main class="body">

      <!-- ─── General: Providers ─── -->
      <template v-if="activeTab === 'general'">
        <!-- Providers -->
        <EditableCardList
          :items="appConfig.providers"
          :title="t('settings.providers')"
          :icon="Settings2"
          :empty-message="t('settings.noProvidersYet')"
          :empty-sub-message="t('settings.addOneToGetStarted')"
          :empty-icon="CircleDot"
          :validate="validateProvider"
          @add="onProviderAdd"
          @confirm="onProviderConfirm"
          @cancel="onProviderCancel"
          @remove="onProviderRemove"
          @drag-end="onProviderDragEnd"
        >
          <template #collapsed="{ item }">
            <div class="prov-lhs">
              <div class="prov-accent" />
              <div class="prov-meta">
                <span class="prov-name" :class="{ dim: !item.name }">{{ item.name || t('settings.untitledProvider') }}</span>
                <span class="prov-badge">{{ item.models.length }} {{ t('settings.model') }}</span>
              </div>
            </div>
          </template>

          <template #name-input="{ item, index }">
            <div class="name-row-wrap">
              <input v-model="item.name" :placeholder="t('settings.providerName')" class="fi name-fi" @click.stop />
              <button
                class="preset-mini-btn"
                :class="{ active: item.preset }"
                @click.stop="togglePresetMenu($event, item, index)"
                :title="item.preset ? `${t('settings.preset')}: ${item.preset}` : t('settings.applyPreset')"
              >
                <Wand2 :size="12" :stroke-width="1.8" />
              </button>
            </div>
          </template>

          <template #content="{ item, index }">
            <Teleport to="body">
              <Transition name="drop">
                <div v-if="showPresetMenu && presetMenuIndex === index" class="sel-menu preset-menu" :style="{ top: presetMenuPos.top + 'px', left: presetMenuPos.left + 'px', minWidth: '220px' }">
                  <div class="sel-clip settings-scrollbar">
                    <button
                      v-for="p in providerPresets" :key="p.name"
                      class="sel-opt"
                      :class="{ hit: item.preset === p.name || (!item.preset && p.name === 'Custom') }"
                      @click="applyPreset(item, p)"
                    >
                      <div class="opt-info">
                        <span class="opt-id">{{ p.name === 'Custom' ? t('onboarding.custom') : p.name }}</span>
                        <span v-if="p.base_url" class="opt-src">{{ p.base_url }}</span>
                      </div>
                      <Check
                        v-if="item.preset === p.name || (!item.preset && p.name === 'Custom')"
                        :size="13" :stroke-width="2.5"
                      />
                    </button>
                    <div v-if="providerPresets.length === 0" class="preset-empty">
                      {{ t('settings.noPresetsFound') }}
                    </div>
                  </div>
                </div>
              </Transition>
            </Teleport>

            <!-- hint -->
            <p v-if="!item.preset" class="preset-hint" @click.stop>
              {{ t('settings.openaiCompatHint') }}
            </p>
            <p v-else-if="providerPresets.find(p => p.name === item.preset)?.api_url" class="preset-hint" @click.stop>
              <a :href="providerPresets.find(p => p.name === item.preset)!.api_url" target="_blank" rel="noopener noreferrer" style="color: var(--color-accent); text-decoration: underline; text-underline-offset: 2px;">
                {{ t('settings.getApiKeyAt', { name: item.preset }) }}
              </a>
            </p>

            <!-- fields -->
            <div class="fields">
              <div class="field">
                <label>{{ t('settings.apiKey') }}</label>
                <div class="key-wrap">
                  <input
                    v-model="item.api_key"
                    :type="editStates.get(index)?.keyVisible ? 'text' : 'password'"
                    class="fi key-fi" placeholder="sk-…" @click.stop
                  />
                  <button class="icon-btn-sm" @click.stop="toggleKeyVisibility(index)" :title="editStates.get(index)?.keyVisible ? 'Hide' : 'Show'">
                    <EyeOff v-if="editStates.get(index)?.keyVisible" :size="12" :stroke-width="1.9" />
                    <Eye v-else :size="12" :stroke-width="1.9" />
                  </button>
                  <button
                    class="icon-btn-sm linkish"
                    @click.stop="testConnection(item, index)"
                    :disabled="!item.api_key || testingProvider === index"
                    :title="t('settings.testConnection')"
                  >
                    <Loader2 v-if="testingProvider === index" :size="12" class="spin" :stroke-width="1.9" />
                    <Link2 v-else :size="12" :stroke-width="1.9" />
                  </button>
                </div>
                <Transition name="fade">
                  <span
                    v-if="editStates.get(index)?.status"
                    class="status-pill"
                    :class="{ ok: editStates.get(index)?.status === 'Connected', err: editStates.get(index)?.status !== 'Connected' }"
                  >
                    <span class="status-dot" />
                    {{ editStates.get(index)?.status }}
                  </span>
                </Transition>
              </div>

              <div class="field">
                <label>{{ t('settings.baseUrl') }}</label>
                <input v-model="item.base_url" class="fi" placeholder="https://api.openai.com/v1" @click.stop />
              </div>
            </div>

            <!-- pool -->
            <div class="pool-bar">
              <span class="pool-label">{{ t('settings.models') }} · {{ item.models.length }}</span>
              <div class="pool-actions">
                <button
                  class="pill-btn micro"
                  @click.stop="fetchModels(item, index)"
                  :disabled="!item.api_key || !item.base_url || editStates.get(index)?.fetching"
                >
                  <Loader2 v-if="editStates.get(index)?.fetching" :size="10" class="spin" :stroke-width="2" />
                  <RefreshCw v-else :size="10" :stroke-width="2" />
                  {{ editStates.get(index)?.fetching ? t('settings.fetching') : t('settings.fetch') }}
                </button>
              </div>
            </div>

            <!-- model tags -->
            <div v-if="item.models.length > 0 && getFetchedModels(index).length === 0" class="tags">
              <span v-for="(m, mi) in item.models" :key="mi" class="tag">
                {{ m.id }}
                <button class="tag-x" @click.stop="removeModel(item, +mi)">
                  <Trash2 :size="9" :stroke-width="2" />
                </button>
              </span>
            </div>
            <div v-if="getFetchedModels(index).length > 0" class="tags pickable">
              <button
                v-for="mid in getFetchedModels(index)" :key="mid"
                class="tag"
                :class="{ selected: item.models.some((m: any) => m.id === mid) }"
                @click.stop="toggleModel(item, mid)"
              >
                <Check v-if="item.models.some((m: any) => m.id === mid)" :size="10" :stroke-width="2.5" />
                {{ mid }}
              </button>
            </div>
          </template>
        </EditableCardList>

        <!-- Interface -->
        <div class="section-head mt">
          <span class="section-title"><Monitor :size="13" />{{ t('settings.interface') }}</span>
        </div>
        <div class="card-section">
          <!-- Theme -->
          <div class="card-row">
            <span class="card-label">{{ t('settings.theme') }}</span>
            <div class="theme-toggle compact">
              <button
                v-for="opt in [{ value: 'light', icon: Sun, label: t('settings.light') }, { value: 'dark', icon: Moon, label: t('settings.dark') }, { value: 'system', icon: SunMoon, label: t('settings.system') }]"
                :key="opt.value"
                class="theme-btn"
                :class="{ on: getTheme() === opt.value }"
                @click="setTheme(opt.value as 'light' | 'dark' | 'system')"
              >
                <component :is="opt.icon" :size="13" :stroke-width="1.8" />
                {{ opt.label }}
              </button>
            </div>
          </div>
          <!-- Floating Window Opacity -->
          <div class="card-row">
            <span class="card-label">{{ t('settings.floatingOpacity') }}</span>
            <div class="opacity-row compact">
              <Droplet :size="13" class="opacity-row-icon" />
              <input
                type="range" min="10" max="100" step="1"
                :value="appConfig.floating_opacity"
                @input="appConfig.floating_opacity = +($event.target as HTMLInputElement).value"
                class="opacity-slider"
              />
              <div class="opacity-value-wrap">
                <input
                  type="number" min="10" max="100"
                  :value="appConfig.floating_opacity"
                  @change="appConfig.floating_opacity = Math.min(100, Math.max(10, +($event.target as HTMLInputElement).value || 90))"
                  class="opacity-value-input"
                />
                <span class="opacity-pct">%</span>
                <button
                  v-if="appConfig.floating_opacity !== 90"
                  class="opacity-reset"
                  @click="appConfig.floating_opacity = 90"
                  :title="t('settings.resetToDefault')"
                >
                  <RotateCcw :size="10" :stroke-width="2" />
                </button>
              </div>
            </div>
          </div>
          <!-- Show Shortcut Hint on Launch -->
          <div class="card-row">
            <span class="card-label">{{ t('settings.showShortcutHintLabel') }}</span>
            <button class="about-auto-btn" :class="{ 'toggle-on': appConfig.show_startup_reminder }" @click="appConfig.show_startup_reminder = !appConfig.show_startup_reminder">
              <ToggleRight v-if="appConfig.show_startup_reminder" :size="15" :stroke-width="1.7" />
              <ToggleLeft v-else :size="15" :stroke-width="1.7" />
            </button>
          </div>
          <!-- Language -->
          <div class="card-row">
            <span class="card-label">{{ t('settings.language') }}</span>
            <div class="sel-wrap compact">
              <button ref="appLangBtnRef" class="sel-btn" @click="toggleAppLangMenu()">
                <Languages :size="13" class="sel-btn-icon" />
                <span class="sel-text">{{ currentAppLangLabel }}</span>
                <ChevronDown :size="11" :stroke-width="2" class="sel-arrow" :class="{ rot: showAppLangMenu }" />
              </button>

              <Teleport to="body">
                <Transition name="drop">
                  <div v-if="showAppLangMenu" class="sel-menu" :style="{ top: appLangMenuPos.top + 'px', left: appLangMenuPos.left + 'px', width: appLangMenuPos.width + 'px' }">
                    <div class="sel-clip settings-scrollbar">
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
              </Teleport>
            </div>
          </div>
        </div>

        <!-- History -->
        <div class="section-head mt">
          <span class="section-title"><History :size="13" />{{ t('history.historySettings') }}</span>
        </div>
        <div class="card-section">
          <div class="card-row">
            <span class="card-label">{{ t('history.historyLimit') }}</span>
            <div class="opacity-row compact">
              <input
                type="number" min="10" max="500" step="10"
                :value="appConfig.history_limit"
                @change="appConfig.history_limit = Math.min(500, Math.max(10, +($event.target as HTMLInputElement).value || 50))"
                class="opacity-value-input"
                style="width: 64px;"
              />
            </div>
          </div>
          <div class="card-row">
            <span class="card-label">{{ t('history.clearHistory') }}</span>
            <template v-if="!showHistoryClearConfirm">
              <button class="pill-btn micro" @click="showHistoryClearConfirm = true">
                <Trash2 :size="10" :stroke-width="2" />
                {{ t('history.clearAll') }}
              </button>
            </template>
            <template v-else>
              <div class="flex items-center gap-2">
                <span class="text-[11px] text-[var(--color-text-secondary)]">{{ t('history.clearConfirm') }}</span>
                <button class="pill-btn gold-micro" @click="clearAllHistory().then(() => showHistoryClearConfirm = false)">
                  {{ t('common.confirm') }}
                </button>
                <button class="pill-btn micro" @click="showHistoryClearConfirm = false">{{ t('common.cancel') }}</button>
              </div>
            </template>
          </div>
        </div>

        <!-- About -->
        <div class="section-head mt">
          <span class="section-title"><Info :size="13" />{{ t('settings.about') }}</span>
        </div>
        <div class="about-row">
          <button class="about-row-info" @click="router.push('/settings/about')">
            <img class="about-row-icon" src="/prompit_logo.svg" alt="" />
            <span class="about-row-name">Prompit</span>
            <span class="about-row-ver">v{{ appVersion }}</span>
          </button>
          <div v-if="isTauri" class="about-row-actions">
            <button
              class="pill-btn micro about-update-btn"
              :class="{
                'au-checking': updateStatus === 'checking',
                'au-ok': updateStatus === 'up-to-date',
                'au-has': updateStatus === 'has-update',
                'au-dl': updateStatus === 'preparing' || updateStatus === 'downloading',
                'au-err': updateStatus === 'error',
              }"
              :disabled="['checking','preparing','downloading','installing','restarting'].includes(updateStatus)"
              @click.stop="handleUpdateClick"
            >
              <Loader2 v-if="updateStatus === 'checking'" :size="10" class="spin" :stroke-width="2" />
              <Check v-else-if="updateStatus === 'up-to-date'" :size="10" :stroke-width="2.5" />
              <RefreshCw v-else-if="updateStatus === 'idle' || updateStatus === 'error'" :size="10" :stroke-width="2" />
              <span v-if="updateStatus === 'has-update'" class="au-ver">v{{ updateVersion }}</span>
              <span v-if="updateStatus === 'downloading' && contentLength > 0">{{ Math.round(downloaded / contentLength * 100) }}%</span>
              <span>{{ updateStatus === 'checking' ? t('about.checking')
                : updateStatus === 'up-to-date' ? t('about.upToDate')
                : updateStatus === 'has-update' ? t('about.install')
                : updateStatus === 'preparing' ? t('about.preparing')
                : updateStatus === 'downloading' ? (contentLength > 0 ? '' : t('about.downloading'))
                : updateStatus === 'installing' ? t('about.installing')
                : updateStatus === 'restarting' ? t('about.restarting')
                : updateStatus === 'error' ? (updateError || t('about.checkFailed'))
                : t('about.checkUpdate')
              }}</span>
            </button>
            <button class="about-auto-btn" :class="{ 'toggle-on': autoUpdate }" @click.stop="toggleAutoUpdate" :title="t('about.autoUpdate')">
              <ToggleRight v-if="autoUpdate" :size="15" :stroke-width="1.7" />
              <ToggleLeft v-else :size="15" :stroke-width="1.7" />
            </button>
          </div>
        </div>

        <!-- User Data -->
        <div class="section-head mt">
          <span class="section-title"><Database :size="13" />{{ t('settings.userData') }}</span>
        </div>
        <div class="reset-row">
          <span class="reset-desc">{{ t('settings.reset.description') }}</span>
          <button class="reset-btn" @click="router.push('/settings/reset')">
            <Trash2 :size="11" :stroke-width="1.9" />{{ t('settings.reset.button') }}
          </button>
        </div>
      </template>

      <!-- ─── Translation tab ─── -->
      <template v-if="activeTab === 'translation'">
        <!-- Model selector -->
        <div class="section-head">
          <span class="section-title"><Cpu :size="13" />{{ t('settings.translationModel') }}</span>
        </div>
        <div class="sel-wrap">
          <button
            ref="selBtnRef"
            class="sel-btn"
            :class="{ dead: allFlat.length === 0 }"
            @click="toggleSelMenu()"
          >
            <span class="sel-text">{{ allFlat.length === 0 ? t('settings.noModelsAvailable') : activeLabel }}</span>
            <ChevronDown :size="11" :stroke-width="2" class="sel-arrow" :class="{ rot: showModelSelector }" />
          </button>

          <Teleport to="body">
            <Transition name="drop">
              <div v-if="showModelSelector && allFlat.length > 0" class="sel-menu" :style="{ top: selMenuPos.top + 'px', left: selMenuPos.left + 'px' }">
                <div class="sel-clip settings-scrollbar">
                <div class="sel-menu-inner">
                  <button
                    v-for="e in allFlat" :key="e.pIndex + '-' + e.mIndex"
                    class="sel-opt"
                    :class="{ hit: e.pIndex === appConfig.active_provider_index && e.mIndex === appConfig.active_model_index }"
                    @click="pickModel(e)"
                  >
                    <div class="opt-info">
                      <span class="opt-id">{{ e.id }}</span>
                      <span class="opt-src">{{ e.providerName }}</span>
                    </div>
                    <Check
                      v-if="e.pIndex === appConfig.active_provider_index && e.mIndex === appConfig.active_model_index"
                      :size="13" :stroke-width="2.5"
                    />
                  </button>
                </div>
                </div>
              </div>
            </Transition>
          </Teleport>
        </div>

        <!-- Target Language -->
        <div class="section-head mt">
          <span class="section-title"><Languages :size="13" />{{ t('settings.targetLanguage') }}</span>
        </div>
        <div class="sel-wrap">
          <button
            ref="langBtnRef"
            class="sel-btn lang-btn"
            @click="toggleLangMenu()"
          >
            <span class="sel-text">{{ getLangName(appConfig.target_lang) }}</span>
            <ChevronDown :size="11" :stroke-width="2" class="sel-arrow" :class="{ rot: showLangSelector }" />
          </button>

          <Teleport to="body">
            <Transition name="drop">
              <div v-if="showLangSelector" class="sel-menu lang-menu" :style="{ top: langMenuPos.top + 'px', left: langMenuPos.left + 'px' }">
                <div class="sel-clip settings-scrollbar">
                <draggable
                  :list="langItems"
                  item-key="id"
                  handle=".lang-drag-handle"
                  :force-fallback="true"
                  fallback-class="hidden-drag-ghost"
                  ghost-class="lang-ghost"
                  @end="onLangDragEnd"
                >
                  <template #item="{ element }">
                    <div
                      class="sel-opt lang-opt"
                      :class="{ hit: element.name === appConfig.target_lang }"
                      @click="pickLang(element.name)"
                    >
                      <span class="lang-drag-handle"><GripVertical :size="11" :stroke-width="1.8" /></span>
                      <span class="opt-label">{{ getLangName(element.name) }}</span>
                      <span class="lang-end">
                        <Check v-if="element.name === appConfig.target_lang" :size="13" :stroke-width="2.5" class="lang-item-check" />
                        <button
                          v-if="element.isCustom"
                          class="lang-item-delete"
                          @click.stop="deleteCustomLang(element.name)"
                          :title="t('settings.removeLanguage')"
                        >
                          <Trash2 :size="11" :stroke-width="1.8" />
                        </button>
                      </span>
                    </div>
                  </template>
                </draggable>

                <!-- Add language -->
                <div class="lang-sep"></div>
                <div v-if="showAddLang" class="lang-add-row">
                  <input
                    v-model="newLangInput"
                    class="lang-add-input"
                    :placeholder="t('settings.languageName')"
                    @keydown.enter="addCustomLang"
                    @click.stop
                    ref="langAddInputRef"
                  />
                  <button class="lang-add-confirm" @click="addCustomLang" :disabled="!newLangInput.trim()">
                    <Check :size="12" :stroke-width="2.5" />
                  </button>
                  <button class="lang-add-cancel" @click="showAddLang = false; newLangInput = ''">
                    <X :size="12" :stroke-width="2" />
                  </button>
                </div>
                <button v-else class="lang-add-btn" @click="showAddLang = true">
                  <Plus :size="11" :stroke-width="2" />{{ t('settings.addLanguage') }}
                </button>

                <!-- Restore default order -->
                <button class="lang-restore-btn" @click="restoreDefaultOrder">
                  <RotateCcw :size="10" :stroke-width="1.8" />{{ t('settings.restoreDefaultOrder') }}
                </button>
                </div>
              </div>
            </Transition>
          </Teleport>
        </div>

        <!-- User Dictionary -->
        <div class="section-head mt">
          <span class="section-title"><BookText :size="13" />{{ t('settings.userDictionary') }}</span>
        </div>
        <div class="dict-toggle-row">
          <template v-if="dictStore.hasEntries">
            <label class="persona-check" :class="{ on: appConfig.user_dict_enabled }" @click.stop>
              <input type="checkbox" :checked="appConfig.user_dict_enabled" @change="appConfig.user_dict_enabled = !appConfig.user_dict_enabled" />
              <Check v-if="appConfig.user_dict_enabled" :size="9" :stroke-width="3" />
            </label>
            <span class="dict-toggle-label">{{ appConfig.user_dict_enabled ? t('common.enabled') : t('common.disabled') }}</span>
          </template>
          <template v-else>
            <span class="dict-toggle-label">{{ t('settings.dictEmpty') }}</span>
          </template>
          <button
            class="pill-btn micro dict-edit-btn"
            @click="router.push('/settings/dictionary?tab=translation')"
          >
            <Pencil :size="10" :stroke-width="2" />{{ t('common.edit') }}
          </button>
        </div>

        <!-- Persona -->
        <EditableCardList
          id="persona-section"
          class="mt"
          :items="personaStore.personas"
          :title="t('settings.translationPersona')"
          :icon="UserCircle"
          :empty-message="t('settings.noPersonasYet')"
          :empty-sub-message="t('settings.addOneToCustomize')"
          :validate="validatePersona"
          @add="personaStore.personas.push({ name: '', prompt: '', enabled: false })"
          @confirm="() => persistPersonas()"
        >
          <template #collapsed="{ item, index }">
            <label class="persona-check" :class="{ on: item.enabled }" @click.stop>
              <input type="checkbox" :checked="item.enabled" @change="togglePersona(index)" />
              <Check v-if="item.enabled" :size="9" :stroke-width="3" />
            </label>
            <span class="persona-name">{{ item.name }}</span>
          </template>

          <template #name-input="{ item, index, isAdding }">
            <input v-model="item.name" :placeholder="t('settings.personaName')" class="fi name-fi" @click.stop />
          </template>

          <template #content="{ item, index }">
            <div class="persona-textarea-wrap">
              <textarea
                v-model="item.prompt"
                placeholder="Enter the translation prompt for this persona…"
                class="persona-textarea"
                rows="3"
                @click.stop
                @keydown="handleTextareaKeydown($event, item, index)"
              />
              <button
                v-if="item.prompt.trim()"
                class="persona-wand-btn"
                :class="{ active: optimizingIndex === index }"
                :title="t('settings.optimizePrompt')"
                @click.stop="handleOptimizePrompt(item, index)"
              >
                <Loader2
                  v-if="optimizingIndex === index"
                  :size="12"
                  :stroke-width="1.9"
                  class="spin"
                />
                <Wand2 v-else :size="13" :stroke-width="1.6" />
              </button>
            </div>
          </template>
        </EditableCardList>
      </template>
    </main>
  </div>
</template>

<style scoped>
/* ══════════════════════════════════════
   Design tokens & base
   ══════════════════════════════════════ */
.settings-root {
  height: 100dvh; display: flex; flex-direction: column;
  background: var(--color-bg); color: var(--color-text); overflow: hidden;
  border-radius: 11px;
}
.settings-root.grow-above .settings-header { order: 2; border-bottom: none; border-top: 1px solid var(--color-surface); }
.settings-root.grow-above .tabs { order: 1; border-bottom: none; border-top: 1px solid var(--color-surface); }
.settings-root.grow-above .body { order: 0; }

/* ── Header ── */
.settings-header {
  display: flex; align-items: center; gap: 12px;
  padding: 16px 24px 12px; border-bottom: 1px solid var(--color-surface);
  flex-shrink: 0;
}
.header-title {
  flex: 1; font-size: 15px; font-weight: 700; letter-spacing: -.02em;
  color: var(--color-text); line-height: 1.2;
}
.back-btn {
  display: flex; align-items: center; justify-content: center;
  width: 32px; height: 32px; border-radius: 9px;
  color: var(--color-text-muted); transition: .15s;
}
.back-btn:hover { color: var(--color-text); background: var(--color-surface-hover); }
.close-btn {
  display: flex; align-items: center; justify-content: center;
  width: 32px; height: 32px; border-radius: 9px;
  color: var(--color-text-muted); transition: .15s;
}
.close-btn:hover { color: var(--color-text); background: var(--color-surface-hover); }

/* ── Tabs ── */
.tabs {
  display: flex; gap: 1px; padding: 0 24px;
  border-bottom: 1px solid var(--color-surface); flex-shrink: 0;
}
.tab {
  display: flex; align-items: center; gap: 6px;
  padding: 10px 16px; font-size: 11px; font-weight: 550;
  color: var(--color-text-muted); position: relative;
  transition: color .18s ease; cursor: default;
}
.tab::after {
  content:""; position:absolute; bottom:-1px; left:16px; right:16px;
  height: 1.5px; border-radius: 1px; background: transparent;
  transition: background .18s ease;
}
.tab:hover { color: var(--color-text-secondary); }
.tab.on { color: var(--color-accent); }
.tab.on::after { background: var(--color-accent); }

/* ── Body scroll ── */
.body {
  flex: 1; overflow-y: auto; padding: 10px 24px 16px;
}
.body::-webkit-scrollbar{width:3px}
.body::-webkit-scrollbar-thumb{background:var(--color-scrollbar);border-radius:3px}

/* ── Shared scrollbar (picker & dropdown lists) ── */
.settings-scrollbar::-webkit-scrollbar{width:3px}
.settings-scrollbar::-webkit-scrollbar-track{margin:10px 0}
.settings-scrollbar::-webkit-scrollbar-thumb{background:var(--color-scrollbar);border-radius:3px}

/* ── Section head ── */
.section-head {
  display:flex; align-items:center; justify-content:space-between;
  margin-bottom: 10px;
}
.section-head.mt { margin-top: 18px; }
.section-title {
  display:flex; align-items:center; gap:7px;
  font-size: 11.5px; font-weight: 650; letter-spacing: .01em;
  color: var(--color-text-secondary);
}
.hint {
  font-size: 9.5px; font-weight: 500; color: var(--color-text-muted);
  text-transform: uppercase; letter-spacing: .04em;
}

/* ── Pill button (Add / Fetch / Add model) ── */
.pill-btn {
  display:inline-flex; align-items:center; gap:4px;
  padding: 4px 11px; border-radius: 7px; font-size: 10.5px; font-weight: 550;
  cursor: pointer; border:none; background:none; transition:.15s;
}
.add-pill { color: var(--color-accent-text); }
.add-pill:hover { color: var(--color-accent); background: var(--color-accent-bg); }
.micro { color: var(--color-text-muted); padding: 3px 8px; }
.micro:hover:not(:disabled){ color: var(--color-text-secondary); background: var(--color-surface-hover); }
.micro:disabled{ opacity:.32; cursor:default; }
.gold-micro { color: var(--color-accent-text); }
.gold-micro:hover { color: rgba(212,160,72,.9); background: var(--color-accent-bg); }
.fetch-ok { color: var(--color-success); cursor: default; }
.fetch-ok:hover { color: var(--color-success); background: var(--color-success-bg); }

/* ── Provider collapsed content ── */
.prov-lhs { display:flex; align-items:center; gap:10px; }
.prov-accent {
  width:3px; height:28px; border-radius: 2px;
  background: linear-gradient(180deg, var(--color-accent-border), rgba(212,160,72,.1));
  flex-shrink:0;
}
.prov-meta { display:flex; align-items:center; gap:8px; }
.prov-name {
  font-size: 12.5px; font-weight: 650; letter-spacing: -.01em;
  color: var(--color-text);
}
.prov-name.dim { color: var(--color-text-placeholder); font-style: italic; }
.prov-badge {
  font-size: 9.5px; font-weight: 550; color: var(--color-text-muted);
  background: var(--color-surface-hover); padding: 1px 7px; border-radius: 6px;
}


/* ── Expanded internals ── */
.name-row {
  display:flex; align-items:center; gap:7px; margin-bottom:13px;
}
.name-row-wrap {
  display:flex; align-items:center; gap:4px; flex:1; min-width:0;
}
.name-fi {
  flex:1; min-width:0;
  font-size:14px; font-weight:700; letter-spacing: -.02em;
}

.fields { display:grid; grid-template-columns:1fr; gap:10px; }
.field { display:flex; flex-direction:column; gap:4px; }

/* ── Preset mini button ── */
.preset-mini-btn {
  display: flex; align-items: center; justify-content: center;
  width: 27px; height: 27px; border-radius: 6px; flex-shrink: 0;
  color: var(--color-text-muted); cursor: pointer;
  border: none; background: none; transition: .12s;
}
.preset-mini-btn:hover { color: var(--color-accent); background: var(--color-surface-hover); }
.preset-mini-btn.active { color: var(--color-accent); }
.preset-mini-btn.active:hover { color: var(--color-accent); background: var(--color-accent-bg); }
.preset-empty {
  padding: 12px; font-size: 10.5px; color: var(--color-text-muted);
  text-align: center; font-style: italic;
}
.preset-hint {
  font-size: 10.5px; color: var(--color-text-muted);
  margin: -2px 0 8px 0; line-height: 1.4;
}

label {
  font-size: 9.5px; font-weight: 600; text-transform:uppercase;
  letter-spacing: .055em; color: var(--color-text-muted);
}

.fi {
  width:100%; background: var(--color-surface);
  border: 1px solid var(--color-border); border-radius:7px;
  padding: 7px 11px; font-size: 12px; color: var(--color-text);
  outline:none; transition:border-color .15s, box-shadow .15s;
}
.fi::placeholder{ color: var(--color-text-placeholder); }
.fi:focus{ border-color: var(--color-accent-border); box-shadow: 0 0 0 2px var(--color-accent-bg); }
.fi.full { padding: 8px 12px; }

/* Key input row */
.key-wrap {
  position:relative; display:flex; align-items:center; gap:2px;
}
.key-fi { padding-right:56px !important; }
.icon-btn-sm {
  position:absolute; top:50%; transform:translateY(-50%);
  display:flex; align-items:center; justify-content:center;
  width:25px; height:25px; border-radius:5px;
  color: var(--color-text-muted); cursor:pointer;
  border:none; background:none; transition:.12s;
}
.icon-btn-sm:nth-of-type(1){ right:27px; }
.icon-btn-sm:nth-of-type(2){ right:2px; }
.icon-btn-sm:hover:not(:disabled){ color: var(--color-text); background: var(--color-surface-hover); }
.icon-btn-sm.linkish { color: var(--color-accent-text); }
.icon-btn-sm.linkish:hover:not(:disabled){ color: var(--color-accent); background: var(--color-accent-bg); }
.icon-btn-sm:disabled{ opacity:.34; cursor:default; }

/* Status pill */
.status-pill {
  display:inline-flex; align-items:center; gap:5px;
  margin-top:5px; padding: 2px 9px 2px 7px; border-radius: 5px;
  font-size: 9.5px; font-weight: 600; letter-spacing: .01em;
}
.status-dot {
  width:5px; height:5px; border-radius:50%; flex-shrink:0;
  background: currentColor;
}
.status-pill.ok { color: var(--color-success); background: var(--color-success-bg); }
.status-pill.err { color: var(--color-danger); background: var(--color-danger-bg); }

/* Pool bar */
.pool-bar {
  display:flex; align-items:center; justify-content:space-between;
  margin-top:14px; padding-top:11px;
  border-top: 1px solid var(--color-surface);
}
.pool-label {
  font-size: 9.5px; font-weight: 600; text-transform:uppercase;
  letter-spacing: .055em; color: var(--color-text-muted);
}
.pool-actions { display:flex; align-items:center; gap:5px; }

/* Picker (fetched models) */
.picker {
  margin-top:7px; border: 1px solid var(--color-border);
  border-radius:9px; background: var(--color-surface); overflow:hidden;
}
.picker-scroll {
  max-height:180px; overflow-y:auto; padding:3px;
}
.pick-item {
  display:flex; align-items:center; justify-content:space-between;
  width:100%; padding: 5px 9px; border-radius:5px;
  font-size: 10.5px; font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
  color: var(--color-text-secondary); cursor:pointer;
  border:none; background:none; text-align:left; transition:.1s;
}
.pick-item:hover:not(.dim){ background: var(--color-accent-bg); color: var(--color-text); }
.pick-item.dim{ color: var(--color-text-muted); cursor:default; }
.picker-done {
  display:block; width:100%; padding:5px; font-size:10px;
  color: var(--color-text-muted); text-align:center;
  border-top: 1px solid var(--color-surface);
  background:none; cursor:pointer; transition:color .12s;
}
.picker-done:hover{ color: var(--color-text-secondary); }

/* Tags (pool items) */
.tags {
  display:flex; flex-wrap:wrap; gap:5px; margin-top:9px;
}
.tag {
  display:inline-flex; align-items:center; gap:4px;
  padding: 3px 8px 3px 7px; border-radius:6px;
  background: var(--color-surface); border: 1px solid var(--color-surface-hover);
  font-size: 10px; font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
  color: var(--color-text-secondary); transition:.12s;
}
.tag:hover{ background: var(--color-surface-hover); border-color: var(--color-border); }
.tag-x {
  display:flex; align-items:center; justify-content:center;
  width:15px; height:15px; border-radius:3px;
  color: var(--color-scrollbar); cursor:pointer;
  border:none; background:none; opacity:0; transition:.1s;
}
.tag:hover .tag-x{ opacity:1; }
.tag-x:hover{ color: var(--color-danger); background: var(--color-danger-bg); }

/* Tags: pickable mode */
.tags.pickable .tag {
  cursor: pointer;
  user-select: none;
}
.tags.pickable .tag.selected {
  background: var(--color-accent-bg);
  border-color: var(--color-accent-border);
  color: var(--color-accent);
}

/* ── Model selector (Translation tab) ── */
.sel-wrap { position:relative; }
.sel-btn {
  display:flex; align-items:center; gap:8px; width:100%;
  padding: 9px 13px; border-radius:9px; font-size:12px;
  background: var(--color-surface); border: 1px solid var(--color-scrollbar);
  color: var(--color-text); cursor:pointer; transition:.15s; text-align:left;
}
.sel-btn:hover:not(.dead){ border-color: var(--color-border-hover); background: var(--color-surface); }
.sel-btn.dead{ color: var(--color-text-muted); cursor:default; }
.sel-text {
  flex:1; font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
  font-size: 11.5px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;
}
.lang-btn .sel-text{ font-family: inherit; font-size:12px; }

/* ── Dictionary toggle row ── */
.dict-toggle-row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 2px;
}
.dict-toggle-label {
  font-size: 11.5px;
  color: var(--color-text-muted);
  min-width: 52px;
}
.dict-edit-btn {
  margin-left: auto;
}

.sel-arrow { color: var(--color-text-muted); transition: transform .18s; flex-shrink:0; }
.sel-arrow.rot{ transform: rotate(180deg); }

.sel-menu {
  position:fixed; min-width:230px; max-width:320px; max-height:180px;
  padding: 0; border-radius: 11px;
  background: var(--color-overlay); backdrop-filter: blur(20px) saturate(1.4);
  border: 1px solid var(--color-border);
  box-shadow: 0 16px 40px rgba(0,0,0,.55), 0 0 0 1px var(--color-surface);
  z-index:99999; overflow:hidden;
}
.sel-clip{ max-height:inherit; overflow-y:auto; overflow-x:hidden; padding:5px 7px 5px 5px; }
.sel-menu-inner{ min-height:0; }
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
.opt-label{
  overflow:hidden; text-overflow:ellipsis; white-space:nowrap; min-width:0;
  font-size:11.5px;
}
.opt-info{ display:flex; flex-direction:column; gap:1px; min-width:0; }
.opt-id{
  font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
  font-size: 11.5px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;
}
.opt-src{ font-size: 9px; color: var(--color-text-muted); letter-spacing: .02em; }
.lang-menu .opt-label{ font-size:12px; }
.lang-menu .sel-opt{ font-size:12px; }
.lang-menu { max-height: 340px; }
.lang-opt { gap: 4px; padding: 4px 8px; justify-content: flex-start; user-select: none; -webkit-user-select: none; }
.lang-opt .lang-drag-handle { opacity: 0; transition: opacity .12s; }
.lang-opt:hover .lang-drag-handle { opacity: 1; }
.lang-end { margin-left: auto; display: flex; align-items: center; gap: 2px; flex-shrink: 0; }
.lang-sep { height: 1px; background: var(--color-surface-hover); margin: 4px 8px; }

/* ── Persona name (in collapsed view) ── */
.persona-name {
  font-size: 12.5px; font-weight: 650; letter-spacing: -.01em;
  color: var(--color-text);
}

/* ── Checkbox ── */
.persona-check {
  position: relative; width:18px; height:18px; border-radius:5px;
  display:inline-flex; align-items:center; justify-content:center;
  border: 1.5px solid var(--color-scrollbar); background: var(--color-surface);
  transition: .15s; color: #121210; cursor:pointer; flex-shrink:0;
  z-index: 1;
}
.persona-check input {
  position:absolute; inset:0; opacity:0; cursor:pointer; margin:0;
}
.persona-check.on {
  border-color: var(--color-accent-border); background: var(--color-accent);
}
.persona-check:hover { border-color: var(--color-text-placeholder); }
.persona-check.on:hover { border-color: var(--color-accent); }

/* ── Persona textarea ── */
.persona-textarea {
  width:100%; background: var(--color-surface);
  border: 1px solid var(--color-border); border-radius:7px;
  padding: 9px 11px; font-size: 12px; color: var(--color-text);
  outline:none; transition:border-color .15s, box-shadow .15s;
  resize: vertical; min-height: 60px; font-family: inherit; line-height: 1.5;
}
.persona-textarea::placeholder { color: var(--color-text-placeholder); }
.persona-textarea:focus { border-color: var(--color-accent-border); box-shadow: 0 0 0 2px var(--color-accent-bg); }

/* ── Persona textarea wrapper (wand button) ── */
.persona-textarea-wrap { position: relative; }
.persona-wand-btn {
  position:absolute; top:-11px; right:-11px;
  width:22px; height:22px; border-radius:50%;
  border: 1px solid var(--color-border); background:var(--color-bg);
  color:var(--color-text-muted); cursor:pointer;
  display:inline-flex; align-items:center; justify-content:center;
  opacity:0; transition:opacity .15s, color .15s, background .15s, border-color .15s; z-index:2;
  box-shadow: 0 1px 3px rgba(0,0,0,.1);
}
.persona-textarea-wrap:hover .persona-wand-btn,
.persona-wand-btn.active { opacity:1; }
.persona-wand-btn.active { color:var(--color-accent); border-color:var(--color-accent); }
.persona-wand-btn:hover { color:var(--color-accent); border-color:var(--color-border-hover); }
@keyframes persona-spin{ to{ transform:rotate(360deg)} }
.persona-wand-btn .spin { animation:persona-spin .75s linear infinite; }

/* ── Transitions ── */
.fade-enter-active,.fade-leave-active{ transition:opacity .18s ease; }
.fade-enter-from,.fade-leave-to{ opacity:0; }
.drop-enter-active,.drop-leave-active{ transition:opacity .14s ease,transform .14s ease; }
.drop-enter-from,.drop-leave-to{ opacity:0; transform: translateY(-5px) scale(.967); }

@keyframes spin{ to{ transform: rotate(360deg)} }
.spin{ animation: spin .75s linear infinite; }

/* ── Language dropdown management ── */
.lang-drag-handle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 4px;
  cursor: grab;
  color: var(--color-text-muted);
  flex-shrink: 0;
  transition: color 0.12s, background 0.12s;
}
.lang-drag-handle:hover {
  color: var(--color-text-muted);
  background: var(--color-surface-hover);
}
.lang-drag-handle:active {
  cursor: grabbing;
  color: var(--color-text-secondary);
}
.lang-item-check {
  flex-shrink: 0;
  color: var(--color-accent);
}
.lang-item-delete {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 5px;
  border: none;
  background: none;
  color: var(--color-text-muted);
  cursor: pointer;
  flex-shrink: 0;
  transition: all 0.12s;
  opacity: 0;
}
.lang-opt:hover .lang-item-delete {
  opacity: 1;
}
.lang-item-delete:hover {
  color: var(--color-danger);
  background: var(--color-danger-bg);
}
.lang-ghost {
  opacity: 0.9;
  background: var(--color-accent-bg);
  border-radius: 7px;
  box-shadow: 0 4px 16px rgba(0,0,0,.3);
}
.lang-dragging {
  opacity: 0.25;
}
.hidden-drag-ghost {
  opacity: 0 !important;
  pointer-events: none !important;
}

.lang-add-row {
  display: flex;
  align-items: center;
  gap: 5px;
  margin-top: 5px;
}
.lang-add-input {
  flex: 1;
  padding: 5px 9px;
  border-radius: 7px;
  border: 1px solid var(--color-border);
  background: var(--color-surface);
  color: var(--color-text);
  font-size: 12px;
  outline: none;
  transition: border-color 0.15s;
}
.lang-add-input:focus {
  border-color: var(--color-accent-border);
}
.lang-add-input::placeholder {
  color: var(--color-text-muted);
}
.lang-add-confirm,
.lang-add-cancel {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border-radius: 7px;
  border: none;
  cursor: pointer;
  flex-shrink: 0;
  transition: all 0.12s;
}
.lang-add-confirm {
  background: var(--color-accent-bg);
  color: var(--color-accent-text);
}
.lang-add-confirm:hover:not(:disabled) {
  background: var(--color-accent-border);
}
.lang-add-confirm:disabled {
  opacity: 0.25;
  cursor: default;
}
.lang-add-cancel {
  background: var(--color-surface);
  color: var(--color-text-placeholder);
}
.lang-add-cancel:hover {
  background: var(--color-scrollbar);
  color: var(--color-text-secondary);
}
.lang-add-btn {
  display: flex;
  align-items: center;
  gap: 5px;
  margin-top: 6px;
  padding: 4px 9px;
  border-radius: 7px;
  border: none;
  background: none;
  color: var(--color-text-muted);
  font-size: 10.5px;
  cursor: pointer;
  transition: all 0.12s;
}
.lang-add-btn:hover {
  color: var(--color-text-secondary);
  background: var(--color-surface);
}
.lang-restore-btn {
  display: flex;
  align-items: center;
  gap: 5px;
  margin-top: 6px;
  padding: 4px 9px;
  border-radius: 7px;
  border: none;
  background: none;
  color: var(--color-text-muted);
  font-size: 10.5px;
  cursor: pointer;
  transition: all 0.12s;
}
.lang-restore-btn:hover {
  color: var(--color-text-secondary);
  background: var(--color-surface);
}

/* ── Theme toggle ── */
.theme-toggle {
  display: flex;
  gap: 1px;
  background: var(--color-border);
  border-radius: 9px;
  padding: 1px;
  margin-bottom: 2px;
}
.theme-btn {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  padding: 7px 12px;
  border-radius: 7px;
  font-size: 11px;
  font-weight: 550;
  color: var(--color-text-muted);
  background: transparent;
  border: none;
  cursor: pointer;
  transition: all 0.15s ease;
}
.theme-btn:hover {
  color: var(--color-text-secondary);
}
.theme-btn.on {
  color: var(--color-text);
  background: var(--color-surface);
  box-shadow: 0 1px 3px rgba(0,0,0,0.08);
}

/* ── Floating opacity slider ── */
.opacity-row {
  display: flex;
  align-items: center;
  gap: 10px;
  background: var(--color-surface);
  border: 1px solid var(--color-scrollbar);
  border-radius: 9px;
  padding: 10px 14px;
}
.opacity-slider {
  flex: 1;
  -webkit-appearance: none;
  appearance: none;
  height: 4px;
  border-radius: 2px;
  background: var(--color-border);
  outline: none;
}
.opacity-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--color-accent);
  cursor: pointer;
  border: 2px solid var(--color-surface);
  box-shadow: 0 1px 4px rgba(0,0,0,0.15);
  transition: transform 0.12s ease;
}
.opacity-slider::-webkit-slider-thumb:hover {
  transform: scale(1.15);
}
.opacity-value-wrap {
  display: flex;
  align-items: center;
  gap: 2px;
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: 6px;
  padding: 3px 6px;
  flex-shrink: 0;
}
.opacity-value-input {
  width: 32px;
  font-size: 11px;
  font-weight: 600;
  text-align: center;
  color: var(--color-text);
  background: transparent;
  border: none;
  outline: none;
  -moz-appearance: textfield;
}
.opacity-value-input::-webkit-outer-spin-button,
.opacity-value-input::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
}
.opacity-pct {
  font-size: 10px;
  font-weight: 600;
  color: var(--color-text-muted);
}
.opacity-reset {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border-radius: 4px;
  border: none;
  background: transparent;
  color: var(--color-text-muted);
  cursor: pointer;
  margin-left: 2px;
  transition: all 0.12s ease;
}
.opacity-reset:hover {
  background: var(--color-border);
  color: var(--color-text-secondary);
}

/* ── Card section: reusable grouped-settings container ── */
/* Usage: section-head (title) + card-section (rows inside a card) */
.card-section {
  display: flex; flex-direction: column; gap: 8px;
  background: var(--color-surface);
  border: 1px solid var(--color-scrollbar);
  border-radius: 10px;
  padding: 12px 14px;
}
.card-row {
  display: flex; align-items: center; justify-content: space-between;
  gap: 12px;
}
.card-row + .card-row {
  border-top: 1px solid var(--color-border);
  padding-top: 8px;
}
.card-label {
  font-size: 11px; font-weight: 500;
  color: var(--color-text-muted);
  white-space: nowrap; flex-shrink: 0;
}
.theme-toggle.compact {
  flex: 0 1 auto;
  margin-bottom: 0;
  width: 240px;
}
.theme-toggle.compact .theme-btn {
  padding: 5px 0;
}
.opacity-row.compact {
  width: 240px;
  flex: none;
  padding: 5px 0;
  background: none;
  border: none;
}
.opacity-row-icon {
  color: var(--color-text-muted);
  flex-shrink: 0;
}
.sel-wrap.compact {
  width: 240px;
}
.sel-wrap.compact .sel-btn {
  width: 100%;
  padding: 5px 10px;
  font-size: 11px;
}
.sel-btn-icon {
  color: var(--color-text-muted);
  flex-shrink: 0;
}

/* ── About row ── */
.about-row {
  display: flex;
  align-items: center;
  gap: 6px;
  background: var(--color-surface);
  border: 1px solid var(--color-scrollbar);
  border-radius: 9px;
  padding: 10px 14px 10px 12px;
}

/* ── Reset row ── */
.reset-row {
  display: flex;
  align-items: center;
  gap: 8px;
  background: var(--color-surface);
  border: 1px solid var(--color-scrollbar);
  border-radius: 9px;
  padding: 10px 14px 10px 14px;
}
.reset-desc {
  flex: 1;
  font-size: 11.5px;
  color: var(--color-text-muted);
}
.reset-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 8px;
  border-radius: 7px;
  font-size: 10.5px;
  font-weight: 550;
  color: var(--color-text-muted);
  cursor: pointer;
  border: none;
  background: none;
  transition: 0.15s;
}
.reset-btn:hover {
  color: var(--color-danger);
  background: var(--color-danger-bg);
}
.about-row-info {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 0;
  border: none;
  background: none;
  cursor: pointer;
  padding: 4px 6px;
  border-radius: 6px;
  transition: 0.15s;
  text-align: left;
}
.about-row-info:hover {
  background: var(--color-surface-hover);
}
.about-row-icon {
  height: 1.2em;
  width: auto;
  flex-shrink: 0;
}
.about-row-name {
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text);
  letter-spacing: -0.01em;
}
.about-row-ver {
  font-size: 10.5px;
  color: var(--color-text-muted);
  font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
  flex-shrink: 0;
}
.about-row-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  flex-shrink: 0;
}
.about-update-btn {
  font-variant-numeric: tabular-nums;
  min-width: 0;
}
.about-update-btn.au-checking {
  color: var(--color-text-secondary);
  pointer-events: none;
}
.about-update-btn.au-ok {
  color: var(--color-success);
  pointer-events: none;
}
.about-update-btn.au-has {
  color: var(--color-accent);
  background: var(--color-accent-bg);
}
.about-update-btn.au-has:hover {
  color: var(--color-accent);
}
.about-update-btn.au-dl {
  color: var(--color-accent-text);
  pointer-events: none;
}
.about-update-btn.au-err {
  color: var(--color-danger);
  pointer-events: none;
}
.au-ver {
  font-weight: 700;
}
.about-auto-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border-radius: 6px;
  border: none;
  background: none;
  cursor: pointer;
  color: var(--color-text-muted);
  transition: 0.15s;
}
.about-auto-btn:hover {
  background: var(--color-surface-hover);
  color: var(--color-text);
}
.about-auto-btn.toggle-on {
  color: var(--color-accent);
}
.about-auto-btn.toggle-on:hover {
  color: var(--color-accent);
}
</style>
