<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useSettingsWindow } from "../composables/useSettingsWindow";
import { getLangName } from "../constants/languages";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  appConfig,
  personaStore,
  loadDictionary,
  saveDictionary,
  importDictionaryCsv,
  exportDictionaryCsv,
  clearAllDictionaries,
  getOrderedLanguages,
} from "../stores/config";
import type { DictEntry } from "../stores/config";
import { ArrowLeft, Download, Upload, Trash2, Plus, Save, ChevronDown, ChevronUp, Check, X, UserCircle } from "@lucide/vue";

const { t } = useI18n();
const entries = ref<DictEntry[]>([]);
const loading = ref(true);
const router = useRouter();
const { growAbove } = useSettingsWindow();
const saveError = ref("");
const dirty = ref(false);

/* ── Persona helpers ── */
const personaNames = computed(() => personaStore.personas.map(p => p.name));
const personaOptions = computed(() => [null, ...personaNames.value]);
function personaLabel(p: string | undefined): string {
  return p ?? t('dictionary.personaAll');
}
function isPersonaValid(p: string | undefined): boolean {
  if (!p) return true;
  return personaNames.value.includes(p);
}

/* ── Persona dropdown per row ── */
const openPersonaRow = ref<number | null>(null);
const personaDropdownPos = ref({ top: 0, right: 0 });

function togglePersonaDropdown(rowIdx: number, event: MouseEvent) {
  if (openPersonaRow.value === rowIdx) {
    openPersonaRow.value = null;
    return;
  }
  const btn = event.currentTarget as HTMLElement;
  const rect = btn.getBoundingClientRect();
  personaDropdownPos.value = { top: rect.bottom + 4, right: window.innerWidth - rect.right };
  openPersonaRow.value = rowIdx;
}

function selectPersona(rowIdx: number, persona: string | null) {
  if (persona === null) {
    delete entries.value[rowIdx].persona;
  } else {
    entries.value[rowIdx].persona = persona;
  }
  openPersonaRow.value = null;
  dirty.value = true;
}

function closePersonaDropdown(e: MouseEvent) {
  const t = e.target as HTMLElement;
  if (!t.closest(".persona-dropdown") && !t.closest(".persona-btn")) {
    openPersonaRow.value = null;
  }
}

/* ── Sorting ── */
const sortCol = ref<'source' | 'target' | 'persona'>('source');
const sortAsc = ref(true);

const sortedEntries = computed(() => {
  const arr = entries.value.map((e, i) => ({ entry: e, origIdx: i }));
  arr.sort((a, b) => {
    let cmp = 0;
    const col = sortCol.value;
    if (col === 'persona') {
      const pa = a.entry.persona ?? '';
      const pb = b.entry.persona ?? '';
      cmp = pa.localeCompare(pb);
    } else {
      cmp = (a.entry[col] ?? '').localeCompare(b.entry[col] ?? '');
    }
    return sortAsc.value ? cmp : -cmp;
  });
  return arr;
});

function toggleSort(col: 'source' | 'target' | 'persona') {
  if (sortCol.value === col) {
    sortAsc.value = !sortAsc.value;
  } else {
    sortCol.value = col;
    sortAsc.value = true;
  }
}

/* ── Multi-select ── */
const selectedSet = ref<Set<number>>(new Set());
const hasSelection = computed(() => selectedSet.value.size > 0);
const allSelected = computed(() =>
  entries.value.length > 0 && selectedSet.value.size === sortedEntries.value.length
);

function toggleSelect(origIdx: number) {
  if (selectedSet.value.has(origIdx)) {
    selectedSet.value.delete(origIdx);
  } else {
    selectedSet.value.add(origIdx);
  }
}

function toggleSelectAll() {
  if (allSelected.value) {
    selectedSet.value.clear();
  } else {
    selectedSet.value = new Set(sortedEntries.value.map(s => s.origIdx));
  }
}

function deleteSelected() {
  entries.value = entries.value.filter((_, i) => !selectedSet.value.has(i));
  selectedSet.value.clear();
  dirty.value = true;
  saveError.value = "";
}

/* ── Batch persona change ── */
const showBatchPersona = ref(false);
const batchPersonaPos = ref({ top: 0, left: 0 });

function openBatchPersona(e: MouseEvent) {
  const btn = e.currentTarget as HTMLElement;
  const rect = btn.getBoundingClientRect();
  batchPersonaPos.value = { top: rect.bottom + 4, left: rect.left };
  showBatchPersona.value = true;
}

function applyBatchPersona(persona: string | null) {
  for (const idx of selectedSet.value) {
    if (persona === null) {
      delete entries.value[idx].persona;
    } else {
      entries.value[idx].persona = persona;
    }
  }
  showBatchPersona.value = false;
  dirty.value = true;
}

function closeBatchDropdown(e: MouseEvent) {
  const t = e.target as HTMLElement;
  if (!t.closest(".batch-persona-dropdown") && !t.closest(".batch-persona-btn")) {
    showBatchPersona.value = false;
  }
}

/* ── View-local target language ── */
const viewLang = ref(appConfig.target_lang);
const showLangMenu = ref(false);
const langMenuPos = ref({ top: 0, left: 0 });
const langBtnRef = ref<HTMLButtonElement | null>(null);
const langItems = computed(() =>
  getOrderedLanguages().map(name => ({ id: name, name }))
);

async function pickViewLang(lang: string) {
  if (lang === viewLang.value) { showLangMenu.value = false; return; }
  viewLang.value = lang;
  showLangMenu.value = false;
  dirty.value = false;
  saveError.value = "";
  try {
    entries.value = await loadDictionary(viewLang.value);
  } catch {
    entries.value = [];
  }
}

function toggleLangMenu() {
  showLangMenu.value = !showLangMenu.value;
  if (showLangMenu.value && langBtnRef.value) {
    const r = langBtnRef.value.getBoundingClientRect();
    langMenuPos.value = { top: r.bottom + 5, left: r.left };
  }
}

function closeLangMenu(e: MouseEvent) {
  const t = e.target as HTMLElement;
  if (!t.closest(".sel-menu") && !t.closest(".sel-btn")) {
    showLangMenu.value = false;
  }
}

/* ── Window drag ── */
async function handleDrag(e: MouseEvent) {
  const t = e.target as HTMLElement;
  if (t.closest("textarea, button, input, select, a, .sel-menu")) return;
  showLangMenu.value = false;
  await getCurrentWindow().startDragging();
}

/* ── Add entry ── */
function addEntry() {
  entries.value.push({ source: "", target: "" });
  dirty.value = true;
}

/* ── Validate & Save ── */
async function handleSave() {
  saveError.value = "";
  for (let i = 0; i < entries.value.length; i++) {
    const e = entries.value[i];
    const hasSource = e.source.trim() !== "";
    const hasTarget = e.target.trim() !== "";
    if (!hasSource && !hasTarget) {
      saveError.value = t('dictionary.rowIsEmpty', { n: i + 1 });
      return;
    }
    if (!hasSource) {
      saveError.value = t('dictionary.sourceRequired', { n: i + 1 });
      return;
    }
    if (!hasTarget) {
      saveError.value = t('dictionary.translationRequired', { n: i + 1 });
      return;
    }
  }
  const valid = entries.value
    .filter((e) => e.source.trim() !== "" && e.target.trim() !== "")
    .map((e) => {
      const entry: DictEntry = { source: e.source.trim(), target: e.target.trim() };
      if (e.persona) entry.persona = e.persona;
      return entry;
    });
  try {
    await saveDictionary(viewLang.value, valid);
    dirty.value = false;
  } catch (err) {
    saveError.value = "Failed to save dictionary.";
    console.error("Failed to save dictionary:", err);
  }
}

/* ── Remove entry ── */
function removeEntry(index: number) {
  entries.value.splice(index, 1);
  dirty.value = true;
  saveError.value = "";
}

/* ── Import flow ── */
const showImportMode = ref(false);
const pendingImportPath = ref("");
const showOverwriteWarn = ref(false);
const importMessage = ref("");
const overwriteCountdown = ref(5);
let overwriteTimer: ReturnType<typeof setInterval> | null = null;

function cancelImportMode() {
  showImportMode.value = false;
  showOverwriteWarn.value = false;
  pendingImportPath.value = "";
  if (overwriteTimer) { clearInterval(overwriteTimer); overwriteTimer = null; }
  overwriteCountdown.value = 5;
}

async function requestImport() {
  const filePath = await open({
    multiple: false,
    filters: [{ name: "CSV", extensions: ["csv"] }],
  });
  if (!filePath) return;
  pendingImportPath.value = filePath as string;
  const currentEntries = entries.value;
  const isEmpty = currentEntries.length === 0;
  if (isEmpty) {
    await executeImport("add");
  } else {
    showImportMode.value = true;
  }
}

async function chooseImportMode(mode: "add" | "overwrite") {
  if (mode === "overwrite") {
    showOverwriteWarn.value = true;
    overwriteCountdown.value = 5;
    if (overwriteTimer) clearInterval(overwriteTimer);
    overwriteTimer = setInterval(() => {
      if (overwriteCountdown.value > 0) {
        overwriteCountdown.value--;
      } else {
        clearInterval(overwriteTimer!);
        overwriteTimer = null;
      }
    }, 1000);
    return;
  }
  showImportMode.value = false;
  await executeImport("add");
}

async function confirmOverwrite() {
  showOverwriteWarn.value = false;
  showImportMode.value = false;
  await executeImport("overwrite");
}

async function executeImport(mode: "add" | "overwrite") {
  try {
    const result = await importDictionaryCsv(pendingImportPath.value, mode);
    entries.value = await loadDictionary(viewLang.value);
    dirty.value = false;
    saveError.value = "";
    const langs = result.languages_affected.join(", ");
    importMessage.value = t('dictionary.imported', { n: result.imported, langs });
    setTimeout(() => { importMessage.value = ""; }, 4000);
  } catch (err) {
    console.error("Failed to import dictionary:", err);
  } finally {
    pendingImportPath.value = "";
  }
}

async function handleExport() {
  const now = new Date();
  const ts = [
    String(now.getFullYear()).slice(2),
    String(now.getMonth() + 1).padStart(2, "0"),
    String(now.getDate()).padStart(2, "0"),
    String(now.getHours()).padStart(2, "0"),
    String(now.getMinutes()).padStart(2, "0"),
  ].join("");
  const filePath = await save({
    defaultPath: `Prompit_Translation_UD-${ts}.csv`,
    filters: [{ name: "CSV", extensions: ["csv"] }],
  });
  if (!filePath) return;
  try {
    await exportDictionaryCsv(filePath);
  } catch (err) {
    console.error("Failed to export dictionary:", err);
  }
}

/* ── Clear flow ── */
const pendingClear = ref<"current" | "all" | null>(null);
const clearCountdown = ref(5);
let clearTimer: ReturnType<typeof setInterval> | null = null;

function requestClearCurrent() {
  pendingClear.value = "current";
  startClearCountdown();
}

function requestClearAll() {
  pendingClear.value = "all";
  startClearCountdown();
}

function startClearCountdown() {
  clearCountdown.value = 5;
  if (clearTimer) clearInterval(clearTimer);
  clearTimer = setInterval(() => {
    if (clearCountdown.value > 0) {
      clearCountdown.value--;
    } else {
      clearInterval(clearTimer!);
      clearTimer = null;
    }
  }, 1000);
}

function cancelClear() {
  pendingClear.value = null;
  if (clearTimer) { clearInterval(clearTimer); clearTimer = null; }
  clearCountdown.value = 5;
}

async function confirmClear() {
  if (pendingClear.value === "current") {
    entries.value = [];
    await handleSave();
  } else if (pendingClear.value === "all") {
    await clearAllDictionaries();
    entries.value = [];
    dirty.value = false;
  }
  pendingClear.value = null;
}

/* ── Lifecycle ── */
onMounted(async () => {
  try {
    entries.value = await loadDictionary(viewLang.value);
  } catch {
    entries.value = [];
  }
  loading.value = false;
  document.addEventListener("click", closeLangMenu);
  document.addEventListener("click", closePersonaDropdown);
  document.addEventListener("click", closeBatchDropdown);
});
onUnmounted(() => {
  document.removeEventListener("click", closeLangMenu);
  document.removeEventListener("click", closePersonaDropdown);
  document.removeEventListener("click", closeBatchDropdown);
});
</script>

<template>
  <div class="dict-root" :class="{ 'grow-above': growAbove }" @mousedown="handleDrag">
    <!-- Header -->
    <div class="dict-header">
      <button class="back-btn" @click="router.push('/settings?tab=translation')">
        <ArrowLeft :size="16" />
      </button>
      <span class="header-title">{{ t('dictionary.userDictionary') }}</span>
      <button class="pill-btn micro" @click="requestImport">
        <Download :size="12" />
        <span>{{ t('dictionary.import') }}</span>
      </button>
      <button class="pill-btn micro" @click="handleExport">
        <Upload :size="12" />
        <span>{{ t('dictionary.export') }}</span>
      </button>
    </div>

    <!-- Language selector + Add Entry -->
    <div class="dict-lang-row">
      <button ref="langBtnRef" class="sel-btn lang-sel-btn" @click="toggleLangMenu">
        <span class="sel-text">{{ getLangName(viewLang) }}</span>
        <ChevronDown :size="11" :stroke-width="2" class="sel-arrow" :class="{ rot: showLangMenu }" />
      </button>

      <!-- Batch actions (when selection active) -->
      <template v-if="hasSelection">
        <span class="batch-count">{{ selectedSet.size }}</span>
        <button class="pill-btn micro batch-persona-btn" @click="openBatchPersona">
          <UserCircle :size="12" />
          <span>{{ t('dictionary.batchPersona') }}</span>
        </button>
        <button class="pill-btn micro" @click="deleteSelected" style="color: var(--color-danger)">
          <Trash2 :size="12" />
          <span>{{ t('dictionary.batchDelete') }}</span>
        </button>
      </template>

      <!-- Add Entry (when no selection) -->
      <button v-else class="pill-btn add-pill" style="margin-left: auto" @click="addEntry">
        <Plus :size="12" :stroke-width="2" />
        <span>{{ t('dictionary.addEntry') }}</span>
      </button>
    </div>

    <!-- Table -->
    <div class="dict-table-wrap">
      <div class="dict-table settings-scrollbar">
        <!-- Sticky header row -->
        <div class="dict-row dict-header-row">
          <div class="dict-col col-check">
            <input type="checkbox" class="dict-checkbox" :checked="allSelected" @change="toggleSelectAll" />
          </div>
          <div class="dict-col col-source sortable" @click="toggleSort('source')">
            {{ t('dictionary.source') }}
            <ChevronUp v-if="sortCol === 'source' && sortAsc" :size="10" class="sort-arrow" />
            <ChevronDown v-if="sortCol === 'source' && !sortAsc" :size="10" class="sort-arrow" />
          </div>
          <div class="dict-col col-trans sortable" @click="toggleSort('target')">
            {{ t('dictionary.translation') }}
            <ChevronUp v-if="sortCol === 'target' && sortAsc" :size="10" class="sort-arrow" />
            <ChevronDown v-if="sortCol === 'target' && !sortAsc" :size="10" class="sort-arrow" />
          </div>
          <div class="dict-col col-persona sortable" @click="toggleSort('persona')">
            {{ t('dictionary.persona') }}
            <ChevronUp v-if="sortCol === 'persona' && sortAsc" :size="10" class="sort-arrow" />
            <ChevronDown v-if="sortCol === 'persona' && !sortAsc" :size="10" class="sort-arrow" />
          </div>
          <div class="dict-col col-action"></div>
        </div>

        <!-- Data rows -->
        <div v-for="{ entry, origIdx } in sortedEntries" :key="origIdx" class="dict-row" :class="{ 'persona-invalid': entry.persona && !isPersonaValid(entry.persona) }">
          <div class="dict-col col-check">
            <input type="checkbox" class="dict-checkbox" :checked="selectedSet.has(origIdx)" @change="toggleSelect(origIdx)" />
          </div>
          <div class="dict-col col-source">
            <input
              class="dict-input"
              v-model="entry.source"
              placeholder="..."
              @input="dirty = true"
            />
          </div>
          <div class="dict-col col-trans">
            <input
              class="dict-input"
              v-model="entry.target"
              placeholder="..."
              @input="dirty = true"
            />
          </div>
          <div class="dict-col col-persona">
            <button
              class="persona-btn"
              :class="{ 'persona-missing': entry.persona && !isPersonaValid(entry.persona) }"
              :title="entry.persona && !isPersonaValid(entry.persona) ? t('dictionary.personaNotFound') : ''"
              @click="togglePersonaDropdown(origIdx, $event)"
            >
              <span class="persona-label">{{ personaLabel(entry.persona) }}</span>
              <ChevronDown :size="10" :stroke-width="2" class="sel-arrow" :class="{ rot: openPersonaRow === origIdx }" />
            </button>
          </div>
          <div class="dict-col col-action">
            <button
              class="mini-btn warn"
              @click="removeEntry(origIdx)"
            >
              <Trash2 :size="13" />
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Language selector dropdown -->
    <Teleport to="body">
      <Transition name="drop">
        <div v-if="showLangMenu" class="sel-menu lang-menu" :style="{ top: langMenuPos.top + 'px', left: langMenuPos.left + 'px' }">
          <div class="sel-clip settings-scrollbar">
            <div
              v-for="item in langItems" :key="item.id"
              class="sel-opt lang-opt"
              :class="{ hit: item.name === viewLang }"
              @click="pickViewLang(item.name)"
            >
              <span class="opt-label">{{ getLangName(item.name) }}</span>
              <Check v-if="item.name === viewLang" :size="13" :stroke-width="2.5" class="lang-item-check" />
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>

    <!-- Persona dropdown per row -->
    <Teleport to="body">
      <Transition name="drop">
        <div
          v-if="openPersonaRow !== null"
          class="sel-menu persona-dropdown"
          :style="{ top: personaDropdownPos.top + 'px', right: personaDropdownPos.right + 'px' }"
        >
          <div class="sel-clip settings-scrollbar">
            <div
              v-for="opt in personaOptions"
              :key="opt ?? '__all__'"
              class="sel-opt"
              :class="{ hit: (openPersonaRow !== null && entries[openPersonaRow]?.persona === opt) || (opt === null && openPersonaRow !== null && !entries[openPersonaRow]?.persona) }"
              @click="selectPersona(openPersonaRow!, opt)"
            >
              <span class="opt-label">{{ personaLabel(opt) }}</span>
              <Check
                v-if="openPersonaRow !== null && ((opt === null && !entries[openPersonaRow]?.persona) || entries[openPersonaRow]?.persona === opt)"
                :size="13" :stroke-width="2.5" class="lang-item-check"
              />
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>

    <!-- Batch persona dropdown -->
    <Teleport to="body">
      <Transition name="drop">
        <div
          v-if="showBatchPersona"
          class="sel-menu batch-persona-dropdown"
          :style="{ top: batchPersonaPos.top + 'px', left: batchPersonaPos.left + 'px' }"
        >
          <div class="sel-clip settings-scrollbar">
            <div
              v-for="opt in personaOptions"
              :key="opt ?? '__all__'"
              class="sel-opt"
              @click="applyBatchPersona(opt)"
            >
              <span class="opt-label">{{ personaLabel(opt) }}</span>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>

    <!-- Footer -->
    <div class="dict-footer">
      <span class="footer-count">{{ t('dictionary.entries') }}: {{ entries.length }}</span>
      <span v-if="importMessage" class="footer-import-msg">{{ importMessage }}</span>
      <span v-if="saveError" class="footer-error">{{ saveError }}</span>

      <!-- Clear buttons -->
      <button class="pill-btn micro clear-btn" @click="requestClearCurrent" :disabled="entries.length === 0">
        <Trash2 :size="11" />
        <span>{{ t('dictionary.clearCurrent') }}</span>
      </button>
      <button class="pill-btn micro clear-btn" @click="requestClearAll">
        <Trash2 :size="11" />
        <span>{{ t('dictionary.clearAll') }}</span>
      </button>

      <button class="pill-btn save-btn" :disabled="!dirty" @click="handleSave">
        <Save :size="12" />
        <span>{{ t('common.save') }}</span>
      </button>
    </div>

    <!-- Import mode dialog -->
    <Teleport to="body">
      <Transition name="drop">
        <div v-if="showImportMode" class="modal-overlay" @click.self="cancelImportMode">
          <div class="modal-card">
            <div class="modal-title">{{ t('dictionary.importModeTitle') }}</div>
            <div class="modal-hint">{{ t('dictionary.importModeHint') }}</div>
            <template v-if="!showOverwriteWarn">
              <div class="modal-actions modal-actions--stacked">
                <button class="modal-choice" @click="chooseImportMode('add')">
                  <span class="choice-label">{{ t('dictionary.addToExisting') }}</span>
                  <span class="choice-desc">{{ t('common.cancel') }}</span>
                </button>
                <button class="modal-choice modal-choice--danger" @click="chooseImportMode('overwrite')">
                  <span class="choice-label">{{ t('dictionary.overwritePerLang') }}</span>
                  <span class="choice-desc">{{ t('dictionary.overwriteWarning') }}</span>
                </button>
              </div>
            </template>
            <template v-else>
              <div class="modal-warn-row">
                <span class="remove-warning-text">{{ t('dictionary.overwriteWarning') }}</span>
              </div>
              <div class="modal-actions">
                <button class="mini-btn" :title="t('common.cancel')" @click="cancelImportMode">
                  <X :size="12" :stroke-width="2.5" />
                </button>
                <div class="confirm-with-countdown" :class="{ counting: overwriteCountdown > 0 }">
                  <button
                    class="mini-btn danger-active"
                    :title="overwriteCountdown > 0 ? t('settings.reset.confirmCountdown', { n: overwriteCountdown }) : t('common.confirm')"
                    :class="{ 'confirm-counting': overwriteCountdown > 0 }"
                    :disabled="overwriteCountdown > 0"
                    @click="confirmOverwrite"
                  >
                    <Check :size="12" :stroke-width="2.5" />
                  </button>
                  <span v-if="overwriteCountdown > 0" class="countdown-label">{{ overwriteCountdown }}s</span>
                </div>
              </div>
            </template>
          </div>
        </div>
      </Transition>
    </Teleport>

    <!-- Clear confirm dialog -->
    <Teleport to="body">
      <Transition name="drop">
        <div v-if="pendingClear" class="modal-overlay" @click.self="cancelClear">
          <div class="modal-card">
            <div class="modal-title">
              {{ pendingClear === 'current'
                ? t('dictionary.clearCurrentConfirm', { lang: getLangName(viewLang) })
                : t('dictionary.clearAllConfirm') }}
            </div>
            <div class="modal-warn-row">
              <span class="remove-warning-text">{{ t('dictionary.overwriteWarning') }}</span>
            </div>
            <div class="modal-actions">
              <button class="mini-btn" :title="t('common.cancel')" @click="cancelClear">
                <X :size="12" :stroke-width="2.5" />
              </button>
              <div class="confirm-with-countdown" :class="{ counting: clearCountdown > 0 }">
                <button
                  class="mini-btn danger-active"
                  :title="clearCountdown > 0 ? t('settings.reset.confirmCountdown', { n: clearCountdown }) : t('common.confirm')"
                  :class="{ 'confirm-counting': clearCountdown > 0 }"
                  :disabled="clearCountdown > 0"
                  @click="confirmClear"
                >
                  <Check :size="12" :stroke-width="2.5" />
                </button>
                <span v-if="clearCountdown > 0" class="countdown-label">{{ clearCountdown }}s</span>
              </div>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
/* ══════════════════════════════════════
   Design tokens & base
   ══════════════════════════════════════ */
.dict-root {
  height: 100dvh;
  display: flex;
  flex-direction: column;
  background: var(--color-bg);
  color: var(--color-text);
  overflow: hidden;
  border-radius: 11px;
}
.dict-root.grow-above .dict-header { order: 99; border-bottom: none; border-top: 1px solid var(--color-surface); }

/* ── Header ── */
.dict-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px 24px 12px;
  border-bottom: 1px solid var(--color-surface);
  flex-shrink: 0;
}
.header-title {
  flex: 1;
  font-size: 15px;
  font-weight: 700;
  letter-spacing: -0.02em;
  color: var(--color-text);
  line-height: 1.2;
}
.back-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 9px;
  color: var(--color-text-muted);
  transition: 0.15s;
}
.back-btn:hover {
  color: var(--color-text);
  background: var(--color-surface-hover);
}

/* ── Pill buttons ── */
.pill-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 11px;
  border-radius: 7px;
  font-size: 10.5px;
  font-weight: 550;
  cursor: pointer;
  border: none;
  background: none;
  transition: 0.15s;
}
.add-pill {
  color: var(--color-accent-text);
}
.add-pill:hover {
  color: var(--color-accent);
  background: var(--color-accent-bg);
}
.micro {
  color: var(--color-text-muted);
  padding: 3px 8px;
}
.micro:hover:not(:disabled) {
  color: var(--color-text-secondary);
  background: var(--color-surface-hover);
}

/* ── Language row ── */
.dict-lang-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 24px 6px;
  flex-shrink: 0;
}

/* ── Language selector (mirrors Settings.vue) ── */
.lang-sel-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 12px;
  border-radius: 9px;
  font-size: 12px;
  background: var(--color-surface);
  border: 1px solid var(--color-scrollbar);
  color: var(--color-text);
  cursor: pointer;
  transition: .15s;
}
.lang-sel-btn:hover {
  border-color: var(--color-border-hover);
}
.lang-sel-btn .sel-text {
  font-family: inherit;
  font-size: 12px;
}
.sel-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.sel-arrow {
  color: var(--color-text-muted);
  transition: transform .18s;
  flex-shrink: 0;
}
.sel-arrow.rot {
  transform: rotate(180deg);
}
.sel-menu {
  position: fixed;
  min-width: 200px;
  max-height: 200px;
  padding: 0;
  border-radius: 11px;
  background: var(--color-overlay);
  backdrop-filter: blur(20px) saturate(1.4);
  border: 1px solid var(--color-border);
  box-shadow: 0 16px 40px rgba(0,0,0,.55), 0 0 0 1px var(--color-surface);
  z-index: 99999;
  overflow: hidden;
}
.sel-clip {
  max-height: inherit;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 5px 7px 5px 5px;
}
.sel-opt {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  width: 100%;
  padding: 8px 11px;
  border-radius: 7px;
  font-size: 12px;
  color: var(--color-text-secondary);
  cursor: pointer;
  border: none;
  background: none;
  text-align: left;
  transition: .1s;
}
.sel-opt:hover {
  background: var(--color-surface-hover);
  color: var(--color-text);
}
.sel-opt.hit {
  background: var(--color-accent-bg);
  color: var(--color-accent-text);
}
.opt-label {
  font-family: inherit;
}
.lang-item-check {
  color: var(--color-accent);
  flex-shrink: 0;
}

/* ── Table ── */
.dict-table-wrap {
  flex: 1;
  overflow: hidden;
  padding: 0 24px;
  min-height: 0;
}
.dict-table {
  height: 100%;
  overflow-y: auto;
  border: 1px solid var(--color-border-hover);
  border-radius: 9px;
}

/* scrollbar */
.settings-scrollbar::-webkit-scrollbar {
  width: 3px;
}
.settings-scrollbar::-webkit-scrollbar-track {
  margin: 10px 0;
}
.settings-scrollbar::-webkit-scrollbar-thumb {
  background: var(--color-scrollbar);
  border-radius: 3px;
}

/* ── Rows ── */
.dict-row {
  display: flex;
  align-items: stretch;
  border-bottom: 1px solid var(--color-border-hover);
}
.dict-row:last-child {
  border-bottom: none;
}

/* ── Header row ── */
.dict-header-row {
  position: sticky;
  top: 0;
  z-index: 1;
  background: var(--color-surface);
  backdrop-filter: blur(6px);
  border-radius: 8px 8px 0 0;
}
.dict-header-row .dict-col {
  font-size: 10.5px;
  font-weight: 650;
  color: var(--color-text-muted);
  letter-spacing: 0.02em;
  padding: 8px 12px;
}

/* ── Columns ── */
.dict-col {
  display: flex;
  align-items: center;
  padding: 0 12px;
  min-height: 36px;
}
.col-source {
  flex: 1;
  border-right: 1px solid var(--color-border-hover);
}
.col-trans {
  flex: 1;
}
.col-action {
  width: 40px;
  justify-content: center;
  flex-shrink: 0;
}

/* ── Inputs ── */
.dict-input {
  width: 100%;
  background: none;
  border: 1px solid transparent;
  border-radius: 4px;
  color: var(--color-text);
  font-size: 12px;
  padding: 6px 6px;
  outline: none;
  transition: border-color 0.18s;
  font-family: inherit;
}
.dict-input:focus {
  border-color: var(--color-accent-border);
}
.dict-input::placeholder {
  color: var(--color-scrollbar);
}

/* ── Mini button (delete) ── */
.mini-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 27px;
  height: 27px;
  border-radius: 7px;
  color: var(--color-text-muted);
  cursor: pointer;
  border: none;
  background: none;
  transition: 0.12s;
}
.mini-btn.warn:hover {
  color: var(--color-danger);
  background: var(--color-danger-bg);
}

/* ── Footer ── */
.dict-footer {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 24px 14px;
  font-size: 10.5px;
  font-weight: 550;
  color: var(--color-text-muted);
  flex-shrink: 0;
}
.footer-count {
  flex: 1;
}
.footer-error {
  color: var(--color-danger);
  font-weight: 500;
  flex: 1;
  text-align: right;
}
.save-btn {
  color: var(--color-text-secondary);
  background: var(--color-accent-bg);
  padding: 4px 12px;
}
.save-btn:hover:not(:disabled) {
  color: var(--color-text);
  background: var(--color-accent-bg);
}
.save-btn:disabled {
  opacity: 0.35;
  cursor: default;
}

/* ── Footer extras ── */
.footer-import-msg {
  flex: 1;
  color: var(--color-accent);
  font-weight: 500;
}
.clear-btn {
  color: var(--color-text-muted);
  margin-left: auto;
}
.clear-btn:hover:not(:disabled) {
  color: var(--color-danger);
  background: var(--color-danger-bg);
}
.clear-btn:disabled {
  opacity: 0.35;
  cursor: default;
}

/* ── Modal overlay ── */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,.35);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 999;
  backdrop-filter: blur(2px);
}
.modal-card {
  background: var(--color-bg);
  border: 1px solid var(--color-surface);
  border-radius: 11px;
  padding: 20px 24px;
  min-width: 280px;
  max-width: 360px;
  box-shadow:
    0 4px 16px rgba(0,0,0,.12),
    0 8px 32px rgba(0,0,0,.14),
    0 2px 6px rgba(0,0,0,.08);
}
.modal-title {
  font-size: 13px;
  font-weight: 700;
  letter-spacing: -0.01em;
  margin-bottom: 4px;
  color: var(--color-text);
  line-height: 1.3;
}
.modal-hint {
  font-size: 11.5px;
  color: var(--color-text-muted);
  margin-bottom: 18px;
  line-height: 1.45;
}
.modal-warn-row {
  margin-bottom: 16px;
  padding: 12px 14px;
  border-radius: 8px;
  background: var(--color-danger-bg);
  border-left: 3px solid var(--color-danger);
  display: flex;
  align-items: center;
  gap: 10px;
}
.modal-warn-row .remove-warning-text {
  display: block;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: .01em;
  color: var(--color-danger);
}
.modal-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}
.modal-actions--stacked {
  flex-direction: column;
  gap: 6px;
}
.modal-choice {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding: 10px 14px;
  border-radius: 8px;
  border: 1px solid var(--color-surface);
  background: var(--color-surface);
  cursor: pointer;
  transition: border-color .15s, background .15s;
  text-align: left;
}
.modal-choice:hover {
  border-color: var(--color-accent-border);
  background: var(--color-bg);
}
.modal-choice--danger {
  border-color: rgba(220,38,38,.25);
  background: rgba(220,38,38,.04);
}
.modal-choice--danger:hover {
  border-color: rgba(220,38,38,.45);
  background: rgba(220,38,38,.08);
}
.choice-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text);
}
.choice-desc {
  font-size: 10px;
  color: var(--color-text-muted);
  margin-left: auto;
  padding-left: 10px;
}
.modal-btn {
  padding: 6px 16px;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: .01em;
  color: var(--color-text-secondary);
  background: var(--color-surface);
  border-radius: 7px;
  transition: color .15s, background .15s;
}
.modal-btn:hover {
  color: var(--color-text);
  background: var(--color-surface-hover);
}
.warn-btn {
  color: var(--color-danger);
  background: var(--color-danger-bg);
}
.warn-btn:hover {
  color: var(--color-danger);
  background: var(--color-danger-bg);
  filter: brightness(.92);
}
.danger-active {
  color: var(--color-danger);
  background: var(--color-danger-bg);
  animation: danger-pulse .8s ease-in-out infinite alternate;
}
.confirm-counting {
  opacity: .55;
  cursor: not-allowed;
  animation: none;
  color: var(--color-text-muted);
  background: var(--color-surface);
}
.confirm-with-countdown {
  display: flex;
  align-items: center;
  gap: 4px;
}
.countdown-label {
  font-size: 10px;
  font-weight: 600;
  color: var(--color-text-muted);
  font-variant-numeric: tabular-nums;
  opacity: .85;
  min-width: 20px;
}
@keyframes danger-pulse {
  to { background: var(--color-danger-bg); filter: brightness(.88); }
}

/* ── Modal transition ── */
.drop-enter-active {
  transition: opacity .18s ease, transform .2s cubic-bezier(.16,1,.3,1);
}
.drop-leave-active {
  transition: opacity .12s ease;
}
.drop-enter-from {
  opacity: 0;
  transform: scale(.96) translateY(4px);
}
.drop-leave-to {
  opacity: 0;
}

/* ── Persona column ── */
.col-persona {
  flex: 0 0 110px;
  border-right: 1px solid var(--color-border-hover);
  border-left: 1px solid var(--color-border-hover);
}
.persona-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 11px;
  color: var(--color-text-secondary);
  background: none;
  border: 1px solid transparent;
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s;
  width: 100%;
}
.persona-btn:hover {
  border-color: var(--color-border-hover);
  background: var(--color-surface-hover);
}
.persona-label {
  flex: 1;
  text-align: left;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.persona-missing .persona-label {
  color: var(--color-text-muted);
  opacity: 0.5;
  text-decoration: line-through;
}
.persona-invalid .dict-input {
  opacity: 0.6;
}

/* ── Sorting ── */
.sortable {
  cursor: pointer;
  user-select: none;
  gap: 4px;
}
.sortable:hover {
  color: var(--color-text-secondary);
}
.sort-arrow {
  color: var(--color-text-muted);
  flex-shrink: 0;
}

/* ── Checkbox column ── */
.col-check {
  width: 32px;
  justify-content: center;
  flex-shrink: 0;
  border-right: 1px solid var(--color-border-hover);
}
.dict-checkbox {
  width: 14px;
  height: 14px;
  cursor: pointer;
  accent-color: var(--color-accent);
}

/* ── Batch actions ── */
.batch-count {
  font-size: 11px;
  font-weight: 600;
  color: var(--color-accent);
  background: var(--color-accent-bg);
  padding: 2px 8px;
  border-radius: 9px;
  margin-left: auto;
}
.batch-persona-btn:hover {
  color: var(--color-accent-text);
  background: var(--color-accent-bg);
}
</style>
