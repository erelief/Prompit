<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { useRouter, useRoute } from "vue-router";
import {
  appConfig,
  personaStore,
  loadConfig,
  saveConfig as persistConfig,
  savePersonas as persistPersonas,
  getOrderedLanguages,
} from "../stores/config";
import type { ProviderConfig } from "../stores/config";
import { BUILTIN_LANGUAGES } from "../constants/languages";
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
} from "@lucide/vue";

type TabKey = "general" | "translation";

const router = useRouter();
const route = useRoute();
const growAbove = ref(false);
const activeTab = ref<TabKey>("general");
const visibleKeys = ref<Set<number>>(new Set());
const testingProvider = ref<number | null>(null);
const fetchStatuses = ref<Map<number, string>>(new Map());
const fetchedModels = ref<Map<string, string[]>>(new Map());
const fetchingProviders = ref(new Set<number>());
const addingModelProvider = ref<number | null>(null);
const showModelSelector = ref(false);
const showLangSelector = ref(false);
const selMenuPos = ref({ top: 0, left: 0 });
const langMenuPos = ref({ top: 0, left: 0 });
const selBtnRef = ref<HTMLElement | null>(null);
const langBtnRef = ref<HTMLElement | null>(null);

// ── Persona management ──
function togglePersona(index: number) {
  const wasOn = personaStore.personas[index].enabled;
  for (const p of personaStore.personas) p.enabled = false;
  if (!wasOn) personaStore.personas[index].enabled = true;
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

  const rv = new Set<number>();
  for (const i of visibleKeys.value) { const m = indexMap.get(i); if (m !== undefined) rv.add(m); }
  visibleKeys.value = rv;

  const rf = new Map<string, string[]>();
  for (const [k, v] of fetchedModels.value) {
    const ni = indexMap.get(parseInt(k.slice(1)));
    if (ni !== undefined) rf.set(`p${ni}`, v);
  }
  fetchedModels.value = rf;

  const rfp = new Set<number>();
  for (const i of fetchingProviders.value) { const m = indexMap.get(i); if (m !== undefined) rfp.add(m); }
  fetchingProviders.value = rfp;

  const rfs = new Map<number, string>();
  for (const [k, v] of fetchStatuses.value) { const m = indexMap.get(k); if (m !== undefined) rfs.set(m, v); }
  fetchStatuses.value = rfs;

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
  const s = new Set(visibleKeys.value);
  s.has(index) ? s.delete(index) : s.add(index);
  visibleKeys.value = s;
}

async function load() {
  try { await loadConfig(); }
  catch (err) { console.error("Failed to load config:", err); }
}

// ── Auto-save ──
let saveTimer: ReturnType<typeof setTimeout> | null = null;
watch(
  () => JSON.stringify(appConfig),
  () => {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => { persistConfig(); }, 800);
  },
);

let personaSaveTimer: ReturnType<typeof setTimeout> | null = null;
watch(
  () => JSON.stringify(personaStore.personas),
  () => {
    if (personaSaveTimer) clearTimeout(personaSaveTimer);
    personaSaveTimer = setTimeout(() => { persistPersonas(); }, 800);
  },
);

function onProviderAdd() {
  appConfig.providers.push({
    name: "", api_key: "",
    base_url: "https://api.openai.com/v1",
    models: [], temperature: 0.3, max_tokens: 1024,
  });
}

function onProviderCancel({ index }: { index: number }) {
  visibleKeys.value.delete(index);
  fetchStatuses.value.delete(index);
  fetchedModels.value.delete(`p${index}`);
  fetchingProviders.value.delete(index);
  if (addingModelProvider.value === index) addingModelProvider.value = null;
  if (testingProvider.value === index) testingProvider.value = null;
}

function onProviderRemove({ index, indexMap }: { index: number; indexMap: Map<number, number> }) {
  visibleKeys.value.delete(index);
  fetchedModels.value.delete(`p${index}`);
  const rv = new Set<number>();
  for (const i of visibleKeys.value) { const m = indexMap.get(i); if (m !== undefined) rv.add(m); }
  visibleKeys.value = rv;
  const rf = new Map<string, string[]>();
  for (const [k, v] of fetchedModels.value) {
    const ni = indexMap.get(parseInt(k.slice(1)));
    if (ni !== undefined) rf.set(`p${ni}`, v);
  }
  fetchedModels.value = rf;
  if (appConfig.active_provider_index >= appConfig.providers.length)
    appConfig.active_provider_index = Math.max(0, appConfig.providers.length - 1);
  const ap = appConfig.providers[appConfig.active_provider_index];
  if (ap && appConfig.active_model_index >= ap.models.length)
    appConfig.active_model_index = Math.max(0, ap.models.length - 1);
}

function removeModel(pIndex: number, mIndex: number) {
  appConfig.providers[pIndex].models.splice(mIndex, 1);
  const p = appConfig.providers[pIndex];
  if (pIndex === appConfig.active_provider_index && appConfig.active_model_index >= p.models.length)
    appConfig.active_model_index = Math.max(0, p.models.length - 1);
}

async function testConnection(provider: ProviderConfig, index: number) {
  if (!provider.api_key || !provider.base_url) return;
  testingProvider.value = index;
  try {
    const url = provider.base_url.replace(/\/v1\/?$/, "").replace(/\/$/, "");
    const r = await fetch(`${url}/models`, {
      method: "GET",
      headers: { Authorization: `Bearer ${provider.api_key}` },
    });
    if (r.ok) {
      fetchStatuses.value.set(index, "Connected");
      setTimeout(() => fetchStatuses.value.delete(index), 3000);
    } else {
      await r.text();
      fetchStatuses.value.set(index, `Failed (${r.status})`);
      setTimeout(() => fetchStatuses.value.delete(index), 4000);
    }
  } catch {
    fetchStatuses.value.set(index, "Connection failed");
    setTimeout(() => fetchStatuses.value.delete(index), 4000);
  } finally {
    testingProvider.value = null;
  }
}

async function fetchModels(provider: ProviderConfig, index: number) {
  if (!provider.api_key || !provider.base_url) return;
  fetchingProviders.value.add(index);
  try {
    const url = provider.base_url.replace(/\/v1\/?$/, "").replace(/\/$/, "");
    const r = await fetch(`${url}/models`, {
      method: "GET",
      headers: { Authorization: `Bearer ${provider.api_key}` },
    });
    if (!r.ok) throw new Error(`HTTP ${r.status}`);
    const data = (await r.json()) as { data?: Array<{ id: string }> };
    fetchedModels.value.set(`p${index}`, data.data?.map((m) => m.id).sort() || []);
  } catch {
    fetchStatuses.value.set(index, "Fetch failed");
    setTimeout(() => fetchStatuses.value.delete(index), 3000);
  } finally {
    fetchingProviders.value.delete(index);
  }
}

function addModelFromList(pi: number, mid: string) {
  const p = appConfig.providers[pi];
  if (!p || p.models.some((m) => m.id === mid)) return;
  p.models.push({ id: mid });
  if (appConfig.providers.reduce((s, p) => s + p.models.length, 0) === 1) {
    appConfig.active_provider_index = pi;
    appConfig.active_model_index = 0;
  }
  addingModelProvider.value = null;
}

function getFetchedModels(pi: number): string[] {
  return fetchedModels.value.get(`p${pi}`) || [];
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
  if (!t.closest(".lang-menu") && !t.closest(".lang-btn"))
    showLangSelector.value = false;
  if (!t.closest(".picker") && !t.closest(".gold-micro"))
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

let unlistenConfig: (() => void) | null = null;

onMounted(async () => {
  if (route.query.tab === "translation") {
    activeTab.value = "translation";
  }
  document.addEventListener("mousedown", onDocClick);
  growAbove.value = await invoke<boolean>("get_grow_above");
  unlistenConfig = await listen<boolean>("window-config", (e) => {
    growAbove.value = e.payload;
  });
  await invoke("resize_and_reposition", { height: 520, width: 480 });
  load();
});

onUnmounted(() => {
  document.removeEventListener("mousedown", onDocClick);
  unlistenConfig?.();
});
</script>

<template>
  <div class="settings-root" :class="{ 'grow-above': growAbove }" @mousedown="handleDrag">
    <!-- ═══ Header ═══ -->
    <header class="settings-header">
      <button @click="goBack" class="back-btn" title="Back">
        <ArrowLeft :size="18" :stroke-width="1.8" />
      </button>
      <h1 class="header-title">Settings</h1>
      <button @click="closeWindow" class="close-btn" title="Close">
        <X :size="16" :stroke-width="1.8" />
      </button>
    </header>

    <!-- ═══ Tabs ═══ -->
    <nav class="tabs">
      <button
        v-for="tab in [{ key: 'general' as TabKey, label: 'General', icon: Settings2 }, { key: 'translation' as TabKey, label: 'Translation', icon: Languages }]"
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
          title="Providers"
          :icon="Settings2"
          empty-message="No providers yet."
          empty-sub-message="Add one to get started."
          :empty-icon="CircleDot"
          @add="onProviderAdd"
          @cancel="onProviderCancel"
          @remove="onProviderRemove"
          @drag-end="onProviderDragEnd"
        >
          <template #collapsed="{ item }">
            <div class="prov-lhs">
              <div class="prov-accent" />
              <div class="prov-meta">
                <span class="prov-name" :class="{ dim: !item.name }">{{ item.name || 'Untitled Provider' }}</span>
                <span class="prov-badge">{{ item.models.length }} model{{ item.models.length !== 1 ? 's' : '' }}</span>
              </div>
            </div>
          </template>

          <template #content="{ item, index }">
            <!-- fields -->
            <div class="fields">
              <div class="field">
                <label>API Key</label>
                <div class="key-wrap">
                  <input
                    v-model="item.api_key"
                    :type="visibleKeys.has(index) ? 'text' : 'password'"
                    class="fi key-fi" placeholder="sk-…" @click.stop
                  />
                  <button class="icon-btn-sm" @click.stop="toggleKeyVisibility(index)" :title="visibleKeys.has(index) ? 'Hide' : 'Show'">
                    <EyeOff v-if="visibleKeys.has(index)" :size="12" :stroke-width="1.9" />
                    <Eye v-else :size="12" :stroke-width="1.9" />
                  </button>
                  <button
                    class="icon-btn-sm linkish"
                    @click.stop="testConnection(item, index)"
                    :disabled="!item.api_key || testingProvider === index"
                    title="Test connection"
                  >
                    <Loader2 v-if="testingProvider === index" :size="12" class="spin" :stroke-width="1.9" />
                    <Link2 v-else :size="12" :stroke-width="1.9" />
                  </button>
                </div>
                <Transition name="fade">
                  <span
                    v-if="fetchStatuses.get(index)"
                    class="status-pill"
                    :class="{ ok: fetchStatuses.get(index) === 'Connected', err: fetchStatuses.get(index) !== 'Connected' }"
                  >
                    <span class="status-dot" />
                    {{ fetchStatuses.get(index) }}
                  </span>
                </Transition>
              </div>

              <div class="field">
                <label>Base URL</label>
                <input v-model="item.base_url" class="fi" placeholder="https://api.openai.com/v1" @click.stop />
              </div>
            </div>

            <!-- pool -->
            <div class="pool-bar">
              <span class="pool-label">Models · {{ item.models.length }}</span>
              <div class="pool-actions">
                <button
                  class="pill-btn micro"
                  @click.stop="fetchModels(item, index)"
                  :disabled="!item.api_key || fetchingProviders.has(index)"
                >
                  <Loader2 v-if="fetchingProviders.has(index)" :size="10" class="spin" :stroke-width="2" />
                  <RefreshCw v-else :size="10" :stroke-width="2" />
                  {{ fetchingProviders.has(index) ? 'Fetching' : 'Fetch' }}
                </button>
                <button
                  v-if="getFetchedModels(index).length > 0 && addingModelProvider !== index"
                  class="pill-btn micro gold-micro"
                  @click.stop="addingModelProvider = index"
                >
                  <Plus :size="10" :stroke-width="2" />Add
                </button>
              </div>
            </div>

            <!-- fetched picker -->
            <div v-if="addingModelProvider === index" class="picker" @click.stop>
              <div class="picker-scroll settings-scrollbar">
                <button
                  v-for="mid in getFetchedModels(index)" :key="mid"
                  class="pick-item"
                  :class="{ dim: item.models.some((m: any) => m.id === mid) }"
                  @click="addModelFromList(index, mid)"
                >
                  <span>{{ mid }}</span>
                  <Check v-if="item.models.some((m: any) => m.id === mid)" :size="11" :stroke-width="2.6" />
                </button>
              </div>
              <button class="picker-done" @click.stop="addingModelProvider = null">Done</button>
            </div>

            <!-- tags -->
            <div v-if="item.models.length > 0" class="tags">
              <span v-for="(m, mi) in item.models" :key="mi" class="tag">
                {{ m.id }}
                <button class="tag-x" @click.stop="removeModel(index, mi)">
                  <Trash2 :size="9" :stroke-width="2" />
                </button>
              </span>
            </div>
          </template>
        </EditableCardList>
      </template>

      <!-- ─── Translation tab ─── -->
      <template v-if="activeTab === 'translation'">
        <!-- Model selector -->
        <div class="section-head">
          <span class="section-title"><Cpu :size="13" />Translation Model</span>
        </div>
        <div class="sel-wrap">
          <button
            ref="selBtnRef"
            class="sel-btn"
            :class="{ dead: allFlat.length === 0 }"
            @click="toggleSelMenu()"
          >
            <span class="sel-text">{{ allFlat.length === 0 ? 'No models available' : activeLabel }}</span>
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
          <span class="section-title"><Languages :size="13" />Target Language</span>
        </div>
        <div class="sel-wrap">
          <button
            ref="langBtnRef"
            class="sel-btn lang-btn"
            @click="toggleLangMenu()"
          >
            <span class="sel-text">{{ appConfig.target_lang }}</span>
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
                      <span class="opt-label">{{ element.name }}</span>
                      <span class="lang-end">
                        <Check v-if="element.name === appConfig.target_lang" :size="13" :stroke-width="2.5" class="lang-item-check" />
                        <button
                          v-if="element.isCustom"
                          class="lang-item-delete"
                          @click.stop="deleteCustomLang(element.name)"
                          title="Remove language"
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
                    placeholder="Language name…"
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
                  <Plus :size="11" :stroke-width="2" />Add language…
                </button>

                <!-- Restore default order -->
                <button class="lang-restore-btn" @click="restoreDefaultOrder">
                  <RotateCcw :size="10" :stroke-width="1.8" />Restore default order
                </button>
                </div>
              </div>
            </Transition>
          </Teleport>
        </div>

        <!-- User Dictionary -->
        <div class="section-head mt">
          <span class="section-title"><BookText :size="13" />User Dictionary</span>
        </div>
        <div class="dict-toggle-row">
          <label class="persona-check" :class="{ on: appConfig.user_dict_enabled }" @click.stop>
            <input type="checkbox" :checked="appConfig.user_dict_enabled" @change="appConfig.user_dict_enabled = !appConfig.user_dict_enabled" />
            <Check v-if="appConfig.user_dict_enabled" :size="9" :stroke-width="3" />
          </label>
          <span class="dict-toggle-label">{{ appConfig.user_dict_enabled ? 'Enabled' : 'Disabled' }}</span>
          <button
            class="pill-btn micro dict-edit-btn"
            @click="router.push('/settings/dictionary?tab=translation')"
          >
            <Pencil :size="10" :stroke-width="2" />Edit
          </button>
        </div>

        <!-- Persona -->
        <EditableCardList
          class="mt"
          :items="personaStore.personas"
          title="Translation Persona"
          :icon="UserCircle"
          empty-message="No personas yet."
          empty-sub-message="Add one to customize translation style."
          @add="personaStore.personas.push({ name: '', prompt: '', enabled: false })"
        >
          <template #collapsed="{ item, index }">
            <label class="persona-check" :class="{ on: item.enabled }" @click.stop>
              <input type="checkbox" :checked="item.enabled" @change="togglePersona(index)" />
              <Check v-if="item.enabled" :size="9" :stroke-width="3" />
            </label>
            <span class="persona-name">{{ item.name }}</span>
          </template>

          <template #name-input="{ item, index, isAdding }">
            <label v-if="!isAdding" class="persona-check" :class="{ on: item.enabled }" @click.stop>
              <input type="checkbox" :checked="item.enabled" @change="togglePersona(index)" />
              <Check v-if="item.enabled" :size="9" :stroke-width="3" />
            </label>
            <input v-model="item.name" placeholder="Persona name…" class="name-input" @click.stop />
          </template>

          <template #content="{ item }">
            <textarea
              v-model="item.prompt"
              placeholder="Enter the translation prompt for this persona…"
              class="persona-textarea"
              rows="3"
              @click.stop
            />
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
  height: 100vh; display: flex; flex-direction: column;
  background: #0b0b0f; color: #fff; overflow: hidden;
  border-radius: 11px;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif;
}
.settings-root.grow-above .settings-header { order: 2; border-bottom: none; border-top: 1px solid rgba(255,255,255,.035); }
.settings-root.grow-above .tabs { order: 1; border-bottom: none; border-top: 1px solid rgba(255,255,255,.035); }
.settings-root.grow-above .body { order: 0; }

/* ── Header ── */
.settings-header {
  display: flex; align-items: center; gap: 12px;
  padding: 16px 24px 12px; border-bottom: 1px solid rgba(255,255,255,.035);
  flex-shrink: 0;
}
.header-title {
  flex: 1; font-size: 15px; font-weight: 700; letter-spacing: -.02em;
  color: rgba(255,255,255,.88); line-height: 1.2;
}
.back-btn {
  display: flex; align-items: center; justify-content: center;
  width: 32px; height: 32px; border-radius: 9px;
  color: rgba(255,255,255,.35); transition: .15s;
}
.back-btn:hover { color: rgba(255,255,255,.75); background: rgba(255,255,255,.06); }
.close-btn {
  display: flex; align-items: center; justify-content: center;
  width: 32px; height: 32px; border-radius: 9px;
  color: rgba(255,255,255,.35); transition: .15s;
}
.close-btn:hover { color: rgba(255,255,255,.75); background: rgba(255,255,255,.06); }

/* ── Tabs ── */
.tabs {
  display: flex; gap: 1px; padding: 0 24px;
  border-bottom: 1px solid rgba(255,255,255,.035); flex-shrink: 0;
}
.tab {
  display: flex; align-items: center; gap: 6px;
  padding: 10px 16px; font-size: 11px; font-weight: 550;
  color: rgba(255,255,255,.28); position: relative;
  transition: color .18s ease; cursor: default;
}
.tab::after {
  content:""; position:absolute; bottom:-1px; left:16px; right:16px;
  height: 1.5px; border-radius: 1px; background: transparent;
  transition: background .18s ease;
}
.tab:hover { color: rgba(255,255,255,.48); }
.tab.on { color: rgba(212,160,72,.85); }
.tab.on::after { background: rgba(212,160,72,.55); }

/* ── Body scroll ── */
.body {
  flex: 1; overflow-y: auto; padding: 10px 24px 16px;
}
.body::-webkit-scrollbar{width:3px}
.body::-webkit-scrollbar-thumb{background:rgba(255,255,255,.07);border-radius:3px}

/* ── Shared scrollbar (picker & dropdown lists) ── */
.settings-scrollbar::-webkit-scrollbar{width:3px}
.settings-scrollbar::-webkit-scrollbar-track{margin:10px 0}
.settings-scrollbar::-webkit-scrollbar-thumb{background:rgba(255,255,255,.12);border-radius:3px}

/* ── Section head ── */
.section-head {
  display:flex; align-items:center; justify-content:space-between;
  margin-bottom: 10px;
}
.section-head.mt { margin-top: 18px; }
.section-title {
  display:flex; align-items:center; gap:7px;
  font-size: 11.5px; font-weight: 650; letter-spacing: .01em;
  color: rgba(255,255,255,.48);
}
.hint {
  font-size: 9.5px; font-weight: 500; color: rgba(255,255,255,.17);
  text-transform: uppercase; letter-spacing: .04em;
}

/* ── Pill button (Add / Fetch / Add model) ── */
.pill-btn {
  display:inline-flex; align-items:center; gap:4px;
  padding: 4px 11px; border-radius: 7px; font-size: 10.5px; font-weight: 550;
  cursor: pointer; border:none; background:none; transition:.15s;
}
.add-pill { color: rgba(212,160,72,.72); }
.add-pill:hover { color: #d4a048; background: rgba(212,160,72,.09); }
.micro { color: rgba(255,255,255,.28); padding: 3px 8px; }
.micro:hover:not(:disabled){ color: rgba(255,255,255,.52); background: rgba(255,255,255,.055); }
.micro:disabled{ opacity:.32; cursor:default; }
.gold-micro { color: rgba(212,160,72,.62); }
.gold-micro:hover { color: rgba(212,160,72,.9); background: rgba(212,160,72,.08); }

/* ── Provider collapsed content ── */
.prov-lhs { display:flex; align-items:center; gap:10px; }
.prov-accent {
  width:3px; height:28px; border-radius: 2px;
  background: linear-gradient(180deg, rgba(212,160,72,.35), rgba(212,160,72,.1));
  flex-shrink:0;
}
.prov-meta { display:flex; align-items:center; gap:8px; }
.prov-name {
  font-size: 12.5px; font-weight: 650; letter-spacing: -.01em;
  color: rgba(255,255,255,.78);
}
.prov-name.dim { color: rgba(255,255,255,.25); font-style: italic; }
.prov-badge {
  font-size: 9.5px; font-weight: 550; color: rgba(255,255,255,.2);
  background: rgba(255,255,255,.055); padding: 1px 7px; border-radius: 6px;
}


/* ── Expanded internals ── */
.name-row {
  display:flex; align-items:center; gap:7px; margin-bottom:13px;
}
.name-input {
  flex:1; background:none; border:none;
  font-size:14px; font-weight:700; letter-spacing: -.02em;
  color: rgba(255,255,255,.86); outline:none;
  padding:3px 5px; border-radius:5px; transition:background .15s;
}
.name-input::placeholder{ color: rgba(255,255,255,.2); }
.name-input:focus{ background: rgba(255,255,255,.045); }

.fields { display:grid; grid-template-columns:1fr 1fr; gap:10px; }
.field { display:flex; flex-direction:column; gap:4px; }
.field:nth-child(1){ grid-column: span 2; }

label {
  font-size: 9.5px; font-weight: 600; text-transform:uppercase;
  letter-spacing: .055em; color: rgba(255,255,255,.26);
}

.fi {
  width:100%; background: rgba(255,255,255,.038);
  border: 1px solid rgba(255,255,255,.065); border-radius:7px;
  padding: 7px 11px; font-size: 12px; color: rgba(255,255,255,.82);
  outline:none; transition:border-color .15s, box-shadow .15s;
}
.fi::placeholder{ color: rgba(255,255,255,.15); }
.fi:focus{ border-color: rgba(212,160,72,.28); box-shadow: 0 0 0 2px rgba(212,160,72,.05); }
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
  color: rgba(255,255,255,.2); cursor:pointer;
  border:none; background:none; transition:.12s;
}
.icon-btn-sm:nth-of-type(1){ right:27px; }
.icon-btn-sm:nth-of-type(2){ right:2px; }
.icon-btn-sm:hover:not(:disabled){ color: rgba(255,255,255,.65); background: rgba(255,255,255,.055); }
.icon-btn-sm.linkish { color: rgba(212,160,72,.42); }
.icon-btn-sm.linkish:hover:not(:disabled){ color: rgba(212,160,72,.85); background: rgba(212,160,72,.09); }
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
.status-pill.ok { color: #4ade80; background: rgba(74,222,128,.08); }
.status-pill.err { color: #f87171; background: rgba(248,113,113,.08); }

/* Pool bar */
.pool-bar {
  display:flex; align-items:center; justify-content:space-between;
  margin-top:14px; padding-top:11px;
  border-top: 1px solid rgba(255,255,255,.038);
}
.pool-label {
  font-size: 9.5px; font-weight: 600; text-transform:uppercase;
  letter-spacing: .055em; color: rgba(255,255,255,.22);
}
.pool-actions { display:flex; align-items:center; gap:5px; }

/* Picker (fetched models) */
.picker {
  margin-top:7px; border: 1px solid rgba(255,255,255,.065);
  border-radius:9px; background: rgba(255,255,255,.018); overflow:hidden;
}
.picker-scroll {
  max-height:180px; overflow-y:auto; padding:3px;
}
.pick-item {
  display:flex; align-items:center; justify-content:space-between;
  width:100%; padding: 5px 9px; border-radius:5px;
  font-size: 10.5px; font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
  color: rgba(255,255,255,.58); cursor:pointer;
  border:none; background:none; text-align:left; transition:.1s;
}
.pick-item:hover:not(.dim){ background: rgba(212,160,72,.07); color: rgba(255,255,255,.9); }
.pick-item.dim{ color: rgba(255,255,255,.13); cursor:default; }
.picker-done {
  display:block; width:100%; padding:5px; font-size:10px;
  color: rgba(255,255,255,.22); text-align:center;
  border-top: 1px solid rgba(255,255,255,.045);
  background:none; cursor:pointer; transition:color .12s;
}
.picker-done:hover{ color: rgba(255,255,255,.45); }

/* Tags (pool items) */
.tags {
  display:flex; flex-wrap:wrap; gap:5px; margin-top:9px;
}
.tag {
  display:inline-flex; align-items:center; gap:4px;
  padding: 3px 8px 3px 7px; border-radius:6px;
  background: rgba(255,255,255,.035); border: 1px solid rgba(255,255,255,.055);
  font-size: 10px; font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
  color: rgba(255,255,255,.48); transition:.12s;
}
.tag:hover{ background: rgba(255,255,255,.052); border-color: rgba(255,255,255,.09); }
.tag-x {
  display:flex; align-items:center; justify-content:center;
  width:15px; height:15px; border-radius:3px;
  color: rgba(255,255,255,.12); cursor:pointer;
  border:none; background:none; opacity:0; transition:.1s;
}
.tag:hover .tag-x{ opacity:1; }
.tag-x:hover{ color:#f87171; background: rgba(248,113,113,.12); }

/* ── Model selector (Translation tab) ── */
.sel-wrap { position:relative; }
.sel-btn {
  display:flex; align-items:center; gap:8px; width:100%;
  padding: 9px 13px; border-radius:9px; font-size:12px;
  background: rgba(255,255,255,.038); border: 1px solid rgba(255,255,255,.07);
  color: rgba(255,255,255,.72); cursor:pointer; transition:.15s; text-align:left;
}
.sel-btn:hover:not(.dead){ border-color: rgba(255,255,255,.115); background: rgba(255,255,255,.05); }
.sel-btn.dead{ color: rgba(255,255,255,.2); cursor:default; }
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
  color: rgba(255, 255, 255, 0.4);
  min-width: 52px;
}
.dict-edit-btn {
  margin-left: auto;
}

.sel-arrow { color: rgba(255,255,255,.22); transition: transform .18s; flex-shrink:0; }
.sel-arrow.rot{ transform: rotate(180deg); }

.sel-menu {
  position:fixed; min-width:230px; max-width:320px; max-height:180px;
  padding: 0; border-radius: 11px;
  background: rgba(16,16,22,.97); backdrop-filter: blur(20px) saturate(1.4);
  border: 1px solid rgba(255,255,255,.075);
  box-shadow: 0 16px 40px rgba(0,0,0,.55), 0 0 0 1px rgba(255,255,255,.025);
  z-index:99999; overflow:hidden;
}
.sel-clip{ max-height:inherit; overflow-y:auto; overflow-x:hidden; padding:5px 7px 5px 5px; }
.sel-menu-inner{ min-height:0; }
.sel-opt {
  display:flex; align-items:center; justify-content:space-between; gap:10px;
  width:100%; padding: 8px 11px; border-radius:7px; font-size:11.5px;
  color: rgba(255,255,255,.52); cursor:pointer;
  border:none; background:none; text-align:left; transition:.1s;
}
.sel-opt:hover{ background: rgba(255,255,255,.055); color: rgba(255,255,255,.85); }
.sel-opt.hit{
  background: rgba(212,160,72,.07); color: rgba(212,160,72,.92);
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
.opt-src{ font-size: 9px; color: rgba(255,255,255,.2); letter-spacing: .02em; }
.lang-menu .opt-label{ font-size:12px; }
.lang-menu .sel-opt{ font-size:12px; }
.lang-menu { max-height: 340px; }
.lang-opt { gap: 4px; padding: 4px 8px; justify-content: flex-start; user-select: none; -webkit-user-select: none; }
.lang-opt .lang-drag-handle { opacity: 0; transition: opacity .12s; }
.lang-opt:hover .lang-drag-handle { opacity: 1; }
.lang-end { margin-left: auto; display: flex; align-items: center; gap: 2px; flex-shrink: 0; }
.lang-sep { height: 1px; background: rgba(255,255,255,.06); margin: 4px 8px; }

/* ── Persona name (in collapsed view) ── */
.persona-name {
  font-size: 12.5px; font-weight: 650; letter-spacing: -.01em;
  color: rgba(255,255,255,.78);
}

/* ── Checkbox ── */
.persona-check {
  position: relative; width:18px; height:18px; border-radius:5px;
  display:inline-flex; align-items:center; justify-content:center;
  border: 1.5px solid rgba(255,255,255,.12); background: rgba(255,255,255,.03);
  transition: .15s; color: #121210; cursor:pointer; flex-shrink:0;
  z-index: 1;
}
.persona-check input {
  position:absolute; inset:0; opacity:0; cursor:pointer; margin:0;
}
.persona-check.on {
  border-color: rgba(212,160,72,.6); background: rgba(212,160,72,.85);
}
.persona-check:hover { border-color: rgba(255,255,255,.25); }
.persona-check.on:hover { border-color: rgba(212,160,72,.9); }

/* ── Persona textarea ── */
.persona-textarea {
  width:100%; background: rgba(255,255,255,.038);
  border: 1px solid rgba(255,255,255,.065); border-radius:7px;
  padding: 9px 11px; font-size: 12px; color: rgba(255,255,255,.82);
  outline:none; transition:border-color .15s, box-shadow .15s;
  resize: vertical; min-height: 60px; font-family: inherit; line-height: 1.5;
}
.persona-textarea::placeholder { color: rgba(255,255,255,.15); }
.persona-textarea:focus { border-color: rgba(212,160,72,.28); box-shadow: 0 0 0 2px rgba(212,160,72,.05); }

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
  color: rgba(255, 255, 255, 0.16);
  flex-shrink: 0;
  transition: color 0.12s, background 0.12s;
}
.lang-drag-handle:hover {
  color: rgba(255, 255, 255, 0.35);
  background: rgba(255, 255, 255, 0.04);
}
.lang-drag-handle:active {
  cursor: grabbing;
  color: rgba(255, 255, 255, 0.5);
}
.lang-item-check {
  flex-shrink: 0;
  color: rgba(212, 160, 72, 0.9);
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
  color: rgba(255, 255, 255, 0.16);
  cursor: pointer;
  flex-shrink: 0;
  transition: all 0.12s;
  opacity: 0;
}
.lang-opt:hover .lang-item-delete {
  opacity: 1;
}
.lang-item-delete:hover {
  color: rgba(239, 68, 68, 0.7);
  background: rgba(239, 68, 68, 0.1);
}
.lang-ghost {
  opacity: 0.9;
  background: rgba(212, 160, 72, 0.08);
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
  border: 1px solid rgba(255, 255, 255, 0.09);
  background: rgba(255, 255, 255, 0.035);
  color: rgba(255, 255, 255, 0.8);
  font-size: 12px;
  outline: none;
  transition: border-color 0.15s;
}
.lang-add-input:focus {
  border-color: rgba(212, 160, 72, 0.35);
}
.lang-add-input::placeholder {
  color: rgba(255, 255, 255, 0.18);
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
  background: rgba(212, 160, 72, 0.1);
  color: rgba(212, 160, 72, 0.8);
}
.lang-add-confirm:hover:not(:disabled) {
  background: rgba(212, 160, 72, 0.18);
}
.lang-add-confirm:disabled {
  opacity: 0.25;
  cursor: default;
}
.lang-add-cancel {
  background: rgba(255, 255, 255, 0.035);
  color: rgba(255, 255, 255, 0.25);
}
.lang-add-cancel:hover {
  background: rgba(255, 255, 255, 0.07);
  color: rgba(255, 255, 255, 0.5);
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
  color: rgba(255, 255, 255, 0.2);
  font-size: 10.5px;
  cursor: pointer;
  transition: all 0.12s;
}
.lang-add-btn:hover {
  color: rgba(255, 255, 255, 0.45);
  background: rgba(255, 255, 255, 0.035);
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
  color: rgba(255, 255, 255, 0.2);
  font-size: 10.5px;
  cursor: pointer;
  transition: all 0.12s;
}
.lang-restore-btn:hover {
  color: rgba(255, 255, 255, 0.45);
  background: rgba(255, 255, 255, 0.035);
}
</style>
