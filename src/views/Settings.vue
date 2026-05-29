<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useRouter } from "vue-router";
import {
  appConfig,
  loadConfig,
  saveConfig as persistConfig,
} from "../stores/config";
import type { ProviderConfig } from "../stores/config";
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
} from "@lucide/vue";

type TabKey = "general" | "translation";

const router = useRouter();
const activeTab = ref<TabKey>("general");
const visibleKeys = ref<Set<number>>(new Set());
const editingProvider = ref<Set<number>>(new Set());
const addingProvider = ref(false);

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
const addingPersona = ref(false);
const editingPersona = ref<Set<number>>(new Set());

function isEditingPersona(index: number): boolean {
  return editingPersona.value.has(index);
}

function toggleEditPersona(index: number) {
  const s = new Set(editingPersona.value);
  s.has(index) ? s.delete(index) : s.add(index);
  editingPersona.value = s;
}

function addPersona() {
  appConfig.personas.push({ name: "", prompt: "", enabled: false });
  addingPersona.value = true;
}

function confirmPersona() {
  addingPersona.value = false;
}

function cancelPersona() {
  appConfig.personas.pop();
  addingPersona.value = false;
}

function removePersona(index: number) {
  appConfig.personas.splice(index, 1);
  const re = new Set<number>();
  for (const i of editingPersona.value) re.add(i > index ? i - 1 : i);
  editingPersona.value = re;
}

function togglePersona(index: number) {
  const wasOn = appConfig.personas[index].enabled;
  for (const p of appConfig.personas) p.enabled = false;
  if (!wasOn) appConfig.personas[index].enabled = true;
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

const targetLanguages = [
  "English", "Simplified Chinese", "Traditional Chinese", "Japanese", "Korean",
  "French", "German", "Spanish", "Russian",
];

function toggleKeyVisibility(index: number) {
  const s = new Set(visibleKeys.value);
  s.has(index) ? s.delete(index) : s.add(index);
  visibleKeys.value = s;
}

function isEditing(pIndex: number): boolean {
  return editingProvider.value.has(pIndex);
}

function toggleEdit(pIndex: number) {
  const s = new Set(editingProvider.value);
  s.has(pIndex) ? s.delete(pIndex) : s.add(pIndex);
  editingProvider.value = s;
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

function addProvider() {
  appConfig.providers.push({
    name: "", api_key: "",
    base_url: "https://api.openai.com/v1",
    models: [], temperature: 0.3, max_tokens: 1024,
  });
  addingProvider.value = true;
}

function confirmProvider() {
  addingProvider.value = false;
}

function cancelProvider() {
  appConfig.providers.pop();
  addingProvider.value = false;
}

function removeProvider(index: number) {
  appConfig.providers.splice(index, 1);
  visibleKeys.value.delete(index);
  editingProvider.value.delete(index);
  fetchedModels.value.delete(`p${index}`);
  const rv = new Set<number>();
  for (const i of visibleKeys.value) rv.add(i > index ? i - 1 : i);
  visibleKeys.value = rv;
  const re = new Set<number>();
  for (const i of editingProvider.value) re.add(i > index ? i - 1 : i);
  editingProvider.value = re;
  const rf = new Map<string, string[]>();
  for (const [k, v] of fetchedModels.value) {
    const n = parseInt(k.slice(1));
    rf.set(`p${n > index ? n - 1 : n}`, v);
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
  await invoke("resize_main_window", { width: 600, height: 200 });
  router.push("/");
}

async function closeWindow() {
  await getCurrentWindow().close();
}

async function handleDrag(e: MouseEvent) {
  const t = e.target as HTMLElement;
  if (t.closest("textarea, button, input, select, a, .provider-card, .sel-menu")) return;
  await getCurrentWindow().startDragging();
}

onMounted(async () => {
  document.addEventListener("mousedown", onDocClick);
  await invoke("resize_main_window", { width: 660, height: 780 });
  load();
});

onUnmounted(() => document.removeEventListener("mousedown", onDocClick));
</script>

<template>
  <div class="settings-root" @mousedown="handleDrag">
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
        <div class="section-head">
          <span class="section-title"><Settings2 :size="13" />Providers</span>
          <button class="pill-btn add-pill" @click="addProvider" :disabled="addingProvider">
            <Plus :size="12" :stroke-width="2" />Add Provider
          </button>
        </div>

        <div class="card-stack">
          <!-- Empty -->
          <div v-if="appConfig.providers.length === 0 && !addingProvider" class="empty-card">
            <CircleDot :size="22" :stroke-width="1" />
            <span>No providers yet.<br><small>Add one to get started.</small></span>
          </div>

          <!-- Adding form -->
          <div v-if="addingProvider" class="prov-card open">
            <div class="prov-expanded">
              <div class="name-row">
                <input
                  v-model="appConfig.providers[appConfig.providers.length - 1].name"
                  placeholder="Provider name…"
                  class="name-input" @click.stop
                />
              </div>
              <div class="fields">
                <div class="field">
                  <label>API Key</label>
                  <div class="key-wrap">
                    <input
                      v-model="appConfig.providers[appConfig.providers.length - 1].api_key"
                      type="password"
                      class="fi key-fi" placeholder="sk-…" @click.stop
                    />
                  </div>
                </div>
              </div>
              <div class="persona-actions">
                <button class="pill-btn gold-micro" @click.stop="confirmProvider">
                  <Check :size="10" :stroke-width="2.5" />Confirm
                </button>
                <button class="pill-btn micro" @click.stop="cancelProvider">Cancel</button>
              </div>
            </div>
          </div>

          <!-- Provider cards -->
          <div
            v-for="(prov, pi) in appConfig.providers.slice(0, addingProvider ? -1 : undefined)"
            :key="pi"
            class="prov-card"
            :class="{ open: isEditing(pi) || !prov.name }"
          >

            <!-- ── Collapsed ── -->
            <div v-if="!isEditing(pi) && prov.name" class="prov-collapsed" @click="toggleEdit(pi)">
              <div class="prov-lhs">
                <div class="prov-accent" />
                <div class="prov-meta">
                  <span class="prov-name">{{ prov.name }}</span>
                  <span class="prov-badge">{{ prov.models.length }} model{{ prov.models.length !== 1 ? 's' : '' }}</span>
                </div>
              </div>
              <div class="prov-rhs" @click.stop>
                <button class="mini-btn" title="Edit" @click="toggleEdit(pi)">
                  <Pencil :size="11" :stroke-width="1.9" />
                </button>
                <button class="mini-btn warn" title="Remove" @click="removeProvider(pi)">
                  <Trash2 :size="11" :stroke-width="1.9" />
                </button>
              </div>
            </div>

            <!-- ── Expanded ── -->
            <div v-else class="prov-expanded">
              <!-- name row -->
              <div class="name-row">
                <input
                  v-model="prov.name" placeholder="Provider name…"
                  class="name-input" @click.stop
                />
                <button v-if="prov.name" class="mini-btn ghost" title="Collapse" @click.stop="toggleEdit(pi)">
                  <ChevronDown :size="14" :stroke-width="1.8" class="chev-up" />
                </button>
                <button class="mini-btn warn" title="Remove" @click.stop="removeProvider(pi)">
                  <Trash2 :size="12" :stroke-width="1.8" />
                </button>
              </div>

              <!-- fields -->
              <div class="fields">
                <div class="field">
                  <label>API Key</label>
                  <div class="key-wrap">
                    <input
                      v-model="prov.api_key"
                      :type="visibleKeys.has(pi) ? 'text' : 'password'"
                      class="fi key-fi" placeholder="sk-…" @click.stop
                    />
                    <button class="icon-btn-sm" @click.stop="toggleKeyVisibility(pi)" :title="visibleKeys.has(pi) ? 'Hide' : 'Show'">
                      <EyeOff v-if="visibleKeys.has(pi)" :size="12" :stroke-width="1.9" />
                      <Eye v-else :size="12" :stroke-width="1.9" />
                    </button>
                    <button
                      class="icon-btn-sm linkish"
                      @click.stop="testConnection(prov, pi)"
                      :disabled="!prov.api_key || testingProvider === pi"
                      title="Test connection"
                    >
                      <Loader2 v-if="testingProvider === pi" :size="12" class="spin" :stroke-width="1.9" />
                      <Link2 v-else :size="12" :stroke-width="1.9" />
                    </button>
                  </div>
                  <Transition name="fade">
                    <span
                      v-if="fetchStatuses.get(pi)"
                      class="status-pill"
                      :class="{ ok: fetchStatuses.get(pi) === 'Connected', err: fetchStatuses.get(pi) !== 'Connected' }"
                    >
                      <span class="status-dot" />
                      {{ fetchStatuses.get(pi) }}
                    </span>
                  </Transition>
                </div>

                <div class="field">
                  <label>Base URL</label>
                  <input v-model="prov.base_url" class="fi" placeholder="https://api.openai.com/v1" @click.stop />
                </div>
              </div>

              <!-- pool -->
              <div class="pool-bar">
                <span class="pool-label">Models · {{ prov.models.length }}</span>
                <div class="pool-actions">
                  <button
                    class="pill-btn micro"
                    @click.stop="fetchModels(prov, pi)"
                    :disabled="!prov.api_key || fetchingProviders.has(pi)"
                  >
                    <Loader2 v-if="fetchingProviders.has(pi)" :size="10" class="spin" :stroke-width="2" />
                    <RefreshCw v-else :size="10" :stroke-width="2" />
                    {{ fetchingProviders.has(pi) ? 'Fetching' : 'Fetch' }}
                  </button>
                  <button
                    v-if="getFetchedModels(pi).length > 0 && addingModelProvider !== pi"
                    class="pill-btn micro gold-micro"
                    @click.stop="addingModelProvider = pi"
                  >
                    <Plus :size="10" :stroke-width="2" />Add
                  </button>
                </div>
              </div>

              <!-- fetched picker -->
              <div v-if="addingModelProvider === pi" class="picker" @click.stop>
                <div class="picker-scroll">
                  <button
                    v-for="mid in getFetchedModels(pi)" :key="mid"
                    class="pick-item"
                    :class="{ dim: prov.models.some(m => m.id === mid) }"
                    @click="addModelFromList(pi, mid)"
                  >
                    <span>{{ mid }}</span>
                    <Check v-if="prov.models.some(m => m.id === mid)" :size="11" :stroke-width="2.6" />
                  </button>
                </div>
                <button class="picker-done" @click.stop="addingModelProvider = null">Done</button>
              </div>

              <!-- tags -->
              <div v-if="prov.models.length > 0" class="tags">
                <span v-for="(m, mi) in prov.models" :key="mi" class="tag">
                  {{ m.id }}
                  <button class="tag-x" @click.stop="removeModel(pi, mi)">
                    <Trash2 :size="9" :stroke-width="2" />
                  </button>
                </span>
              </div>
            </div>
          </div>
        </div>
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
                <div class="sel-clip">
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
                <div class="sel-clip">
                <div class="sel-menu-inner">
                  <button
                    v-for="lang in targetLanguages" :key="lang"
                    class="sel-opt"
                    :class="{ hit: lang === appConfig.target_lang }"
                    @click="pickLang(lang)"
                  >
                    <span class="opt-label">{{ lang }}</span>
                    <Check
                      v-if="lang === appConfig.target_lang"
                      :size="13" :stroke-width="2.5"
                    />
                  </button>
                </div>
                </div>
              </div>
            </Transition>
          </Teleport>
        </div>

        <!-- Persona -->
        <div class="section-head mt">
          <span class="section-title"><UserCircle :size="13" />Translation Persona</span>
          <button class="pill-btn add-pill" @click="addPersona" :disabled="addingPersona">
            <Plus :size="12" :stroke-width="2" />Add Persona
          </button>
        </div>

        <div class="card-stack">
          <!-- Empty -->
          <div v-if="appConfig.personas.length === 0 && !addingPersona" class="empty-card">
            <UserCircle :size="22" :stroke-width="1" />
            <span>No personas yet.<br><small>Add one to customize translation style.</small></span>
          </div>

          <!-- Adding form (no checkbox, Confirm/Cancel) -->
          <div v-if="addingPersona" class="persona-card open">
            <div class="persona-expanded">
              <div class="name-row">
                <input
                  v-model="appConfig.personas[appConfig.personas.length - 1].name"
                  placeholder="Persona name…"
                  class="name-input" @click.stop
                />
              </div>
              <textarea
                v-model="appConfig.personas[appConfig.personas.length - 1].prompt"
                placeholder="Enter the translation prompt for this persona…"
                class="persona-textarea"
                rows="3"
                @click.stop
              />
              <div class="persona-actions">
                <button class="pill-btn gold-micro" @click.stop="confirmPersona">
                  <Check :size="10" :stroke-width="2.5" />Confirm
                </button>
                <button class="pill-btn micro" @click.stop="cancelPersona">Cancel</button>
              </div>
            </div>
          </div>

          <!-- Persona cards (skip last while adding) -->
          <div
            v-for="(persona, psi) in appConfig.personas.slice(0, addingPersona ? -1 : undefined)"
            :key="psi"
            class="persona-card"
            :class="{ open: isEditingPersona(psi) }"
          >
            <!-- Collapsed: checkbox + name only -->
            <div v-if="!isEditingPersona(psi)" class="persona-collapsed" @click="toggleEditPersona(psi)">
              <div class="persona-lhs">
                <label class="persona-check" :class="{ on: persona.enabled }" @click.stop>
                  <input type="checkbox" :checked="persona.enabled" @change="togglePersona(psi)" />
                  <Check v-if="persona.enabled" :size="9" :stroke-width="3" />
                </label>
                <span class="persona-name">{{ persona.name }}</span>
              </div>
              <div class="persona-rhs" @click.stop>
                <button class="mini-btn" title="Edit" @click="toggleEditPersona(psi)">
                  <Pencil :size="11" :stroke-width="1.9" />
                </button>
                <button class="mini-btn warn" title="Remove" @click="removePersona(psi)">
                  <Trash2 :size="11" :stroke-width="1.9" />
                </button>
              </div>
            </div>

            <!-- Expanded: checkbox + name + prompt -->
            <div v-else class="persona-expanded">
              <div class="name-row">
                <label class="persona-check" :class="{ on: persona.enabled }" @click.stop>
                  <input type="checkbox" :checked="persona.enabled" @change="togglePersona(psi)" />
                  <Check v-if="persona.enabled" :size="9" :stroke-width="3" />
                </label>
                <input
                  v-model="persona.name" placeholder="Persona name…"
                  class="name-input" @click.stop
                />
                <button class="mini-btn ghost" title="Collapse" @click.stop="toggleEditPersona(psi)">
                  <ChevronDown :size="14" :stroke-width="1.8" class="chev-up" />
                </button>
                <button class="mini-btn warn" title="Remove" @click.stop="removePersona(psi)">
                  <Trash2 :size="12" :stroke-width="1.8" />
                </button>
              </div>
              <textarea
                v-model="persona.prompt"
                placeholder="Enter the translation prompt for this persona…"
                class="persona-textarea"
                rows="3"
                @click.stop
              />
            </div>
          </div>
        </div>
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

/* ── Card stack ── */
.card-stack { display:flex; flex-direction:column; gap:7px; }

/* ── Provider card ── */
.prov-card {
  border-radius: 11px; overflow:hidden;
  border: 1px solid rgba(255,255,255,.055);
  background: linear-gradient(180deg, rgba(255,255,255,.022) 0%, rgba(255,255,255,.014) 100%);
  transition: border-color .18s, box-shadow .18s;
}
.prov-card:hover { border-color: rgba(255,255,255,.09); }
.prov-card.open { padding: 15px 16px 14px; }

/* ── Collapsed row ── */
.prov-collapsed {
  display:flex; align-items:center; justify-content:space-between;
  padding: 11px 14px; cursor:pointer; transition:background .12s;
}
.prov-collapsed:hover { background: rgba(255,255,255,.02); }
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
.prov-badge {
  font-size: 9.5px; font-weight: 550; color: rgba(255,255,255,.2);
  background: rgba(255,255,255,.055); padding: 1px 7px; border-radius: 6px;
}
.prov-rhs { display:flex; align-items:center; gap:2px; opacity:.6; transition:opacity .12s; }
.prov-collapsed:hover .prov-rhs { opacity:1; }

.mini-btn {
  display:flex; align-items:center; justify-content:center;
  width:27px; height:27px; border-radius:7px;
  color: rgba(255,255,255,.32); cursor:pointer;
  border:none; background:none; transition:.12s;
}
.mini-btn:hover { color: rgba(255,255,255,.7); background: rgba(255,255,255,.065); }
.mini-btn.warn:hover { color: #f87171; background: rgba(248,113,113,.1); }
.mini-btn.ghost { color: rgba(255,255,255,.2); }
.mini-btn.ghost:hover { color: rgba(255,255,255,.48); background: rgba(255,255,255,.045); }

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
  max-height:172px; overflow-y:auto; padding:3px;
}
.picker-scroll::-webkit-scrollbar{width:2.5px}
.picker-scroll::-webkit-scrollbar-thumb{background:rgba(255,255,255,.09);border-radius:3px}
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
.sel-clip::-webkit-scrollbar{width:3px}
.sel-clip::-webkit-scrollbar-thumb{background:rgba(255,255,255,.12);border-radius:3px}
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

/* ── Empty state ── */
.empty-card {
  display:flex; flex-direction:column; align-items:center; gap:8px;
  padding: 28px 16px; border-radius: 11px;
  border: 1px dashed rgba(255,255,255,.06);
  color: rgba(255,255,255,.18); font-size: 11.5px; line-height: 1.5;
  text-align: center;
}
.empty-card small{ font-size: 10px; color: rgba(255,255,255,.13); }

/* ── Persona card ── */
.persona-card {
  border-radius: 11px; overflow:hidden;
  border: 1px solid rgba(255,255,255,.055);
  background: linear-gradient(180deg, rgba(255,255,255,.022) 0%, rgba(255,255,255,.014) 100%);
  transition: border-color .18s, box-shadow .18s;
}
.persona-card:hover { border-color: rgba(255,255,255,.09); }
.persona-card.open { padding: 15px 16px 14px; }

/* ── Persona collapsed ── */
.persona-collapsed {
  display:flex; align-items:center; justify-content:space-between;
  padding: 11px 14px; cursor:pointer; transition:background .12s;
}
.persona-collapsed:hover { background: rgba(255,255,255,.02); }
.persona-lhs { display:flex; align-items:center; gap:10px; min-width:0; flex:1; }
.persona-name {
  font-size: 12.5px; font-weight: 650; letter-spacing: -.01em;
  color: rgba(255,255,255,.78);
}
.persona-rhs { display:flex; align-items:center; gap:2px; opacity:.6; transition:opacity .12s; }
.persona-collapsed:hover .persona-rhs { opacity:1; }

/* ── Persona expanded ── */
.persona-expanded .name-row {
  display:flex; align-items:center; gap:7px; margin-bottom:10px;
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

/* ── Persona actions (Confirm / Cancel) ── */
.persona-actions {
  display:flex; align-items:center; gap:6px; margin-top:10px;
}

/* ── Transitions ── */
.fade-enter-active,.fade-leave-active{ transition:opacity .18s ease; }
.fade-enter-from,.fade-leave-to{ opacity:0; }
.drop-enter-active,.drop-leave-active{ transition:opacity .14s ease,transform .14s ease; }
.drop-enter-from,.drop-leave-to{ opacity:0; transform: translateY(-5px) scale(.967); }

@keyframes spin{ to{ transform: rotate(360deg)} }
.spin{ animation: spin .75s linear infinite; }
.chev-up{ transform: rotate(180deg); }
</style>
