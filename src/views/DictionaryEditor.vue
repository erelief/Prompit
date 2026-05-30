<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  appConfig,
  loadDictionary,
  saveDictionary,
  importDictionaryCsv,
  exportDictionaryCsv,
} from "../stores/config";
import type { DictEntry } from "../stores/config";
import { ArrowLeft, Download, Upload, Trash2, Plus, Save } from "@lucide/vue";

const entries = ref<DictEntry[]>([]);
const loading = ref(true);
const router = useRouter();
const saveError = ref("");
const dirty = ref(false);

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
      saveError.value = `Row ${i + 1} is empty — fill it or delete it.`;
      return;
    }
    if (!hasSource) {
      saveError.value = `Row ${i + 1}: Translation requires a Source.`;
      return;
    }
    if (!hasTarget) {
      saveError.value = `Row ${i + 1}: Source requires a Translation.`;
      return;
    }
  }
  const valid = entries.value
    .filter((e) => e.source.trim() !== "" && e.target.trim() !== "")
    .map((e) => ({ source: e.source.trim(), target: e.target.trim() }));
  try {
    await saveDictionary(appConfig.target_lang, valid);
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

/* ── Import / Export ── */
async function handleImport() {
  const filePath = await open({
    multiple: false,
    filters: [{ name: "CSV", extensions: ["csv"] }],
  });
  if (!filePath) return;
  try {
    await importDictionaryCsv(appConfig.target_lang, filePath as string);
    entries.value = await loadDictionary(appConfig.target_lang);
    dirty.value = false;
    saveError.value = "";
  } catch (err) {
    console.error("Failed to import dictionary:", err);
  }
}

async function handleExport() {
  const filePath = await save({
    defaultPath: `dictionary-${appConfig.target_lang.toLowerCase()}.csv`,
    filters: [{ name: "CSV", extensions: ["csv"] }],
  });
  if (!filePath) return;
  try {
    await exportDictionaryCsv(appConfig.target_lang, filePath);
  } catch (err) {
    console.error("Failed to export dictionary:", err);
  }
}

/* ── Lifecycle ── */
onMounted(async () => {
  try {
    entries.value = await loadDictionary(appConfig.target_lang);
  } catch {
    entries.value = [];
  }
  loading.value = false;
});
</script>

<template>
  <div class="dict-root">
    <!-- Header -->
    <div class="dict-header">
      <button class="back-btn" @click="router.push('/settings?tab=translation')">
        <ArrowLeft :size="16" />
      </button>
      <span class="header-title">User Dictionary</span>
      <button class="pill-btn micro" @click="handleImport">
        <Download :size="12" />
        <span>Import</span>
      </button>
      <button class="pill-btn micro" @click="handleExport">
        <Upload :size="12" />
        <span>Export</span>
      </button>
    </div>

    <!-- Language label + Add Entry -->
    <div class="dict-lang-row">
      <span class="dict-lang">Target: {{ appConfig.target_lang }}</span>
      <button class="pill-btn add-pill" @click="addEntry">
        <Plus :size="12" :stroke-width="2" />
        <span>Add Entry</span>
      </button>
    </div>

    <!-- Table -->
    <div class="dict-table-wrap">
      <div class="dict-table settings-scrollbar">
        <!-- Sticky header row -->
        <div class="dict-row dict-header-row">
          <div class="dict-col col-source">Source</div>
          <div class="dict-col col-trans">Translation</div>
          <div class="dict-col col-action"></div>
        </div>

        <!-- Data rows -->
        <div v-for="(entry, i) in entries" :key="i" class="dict-row">
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
          <div class="dict-col col-action">
            <button
              class="mini-btn warn"
              @click="removeEntry(i)"
            >
              <Trash2 :size="13" />
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Footer -->
    <div class="dict-footer">
      <span class="footer-count">Entries: {{ entries.length }}</span>
      <span v-if="saveError" class="footer-error">{{ saveError }}</span>
      <button class="pill-btn save-btn" :disabled="!dirty" @click="handleSave">
        <Save :size="12" />
        <span>Save</span>
      </button>
    </div>
  </div>
</template>

<style scoped>
/* ══════════════════════════════════════
   Design tokens & base
   ══════════════════════════════════════ */
.dict-root {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: #0b0b0f;
  color: #fff;
  overflow: hidden;
  border-radius: 11px;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui,
    sans-serif;
}

/* ── Header ── */
.dict-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px 24px 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.035);
  flex-shrink: 0;
}
.header-title {
  flex: 1;
  font-size: 15px;
  font-weight: 700;
  letter-spacing: -0.02em;
  color: rgba(255, 255, 255, 0.88);
  line-height: 1.2;
}
.back-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 9px;
  color: rgba(255, 255, 255, 0.35);
  transition: 0.15s;
}
.back-btn:hover {
  color: rgba(255, 255, 255, 0.75);
  background: rgba(255, 255, 255, 0.06);
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
  color: rgba(212, 160, 72, 0.72);
}
.add-pill:hover {
  color: #d4a048;
  background: rgba(212, 160, 72, 0.09);
}
.micro {
  color: rgba(255, 255, 255, 0.28);
  padding: 3px 8px;
}
.micro:hover:not(:disabled) {
  color: rgba(255, 255, 255, 0.52);
  background: rgba(255, 255, 255, 0.055);
}

/* ── Language row ── */
.dict-lang-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 24px 6px;
  flex-shrink: 0;
}
.dict-lang {
  flex: 1;
  font-size: 11.5px;
  font-weight: 550;
  color: rgba(255, 255, 255, 0.36);
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
  border: 1px solid rgba(255, 255, 255, 0.045);
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
  background: rgba(255, 255, 255, 0.12);
  border-radius: 3px;
}

/* ── Rows ── */
.dict-row {
  display: flex;
  align-items: stretch;
  border-bottom: 1px solid rgba(255, 255, 255, 0.035);
}
.dict-row:last-child {
  border-bottom: none;
}

/* ── Header row ── */
.dict-header-row {
  position: sticky;
  top: 0;
  z-index: 1;
  background: rgba(255, 255, 255, 0.025);
  backdrop-filter: blur(6px);
}
.dict-header-row .dict-col {
  font-size: 10.5px;
  font-weight: 650;
  color: rgba(255, 255, 255, 0.38);
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
  border-right: 1px solid rgba(255, 255, 255, 0.035);
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
  color: rgba(255, 255, 255, 0.82);
  font-size: 12px;
  padding: 6px 6px;
  outline: none;
  transition: border-color 0.18s;
  font-family: inherit;
}
.dict-input:focus {
  border-color: rgba(212, 160, 72, 0.3);
}
.dict-input::placeholder {
  color: rgba(255, 255, 255, 0.12);
}

/* ── Mini button (delete) ── */
.mini-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 27px;
  height: 27px;
  border-radius: 7px;
  color: rgba(255, 255, 255, 0.32);
  cursor: pointer;
  border: none;
  background: none;
  transition: 0.12s;
}
.mini-btn.warn:hover {
  color: #f87171;
  background: rgba(248, 113, 113, 0.1);
}

/* ── Footer ── */
.dict-footer {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 24px 14px;
  font-size: 10.5px;
  font-weight: 550;
  color: rgba(255, 255, 255, 0.2);
  flex-shrink: 0;
}
.footer-count {
  flex: 1;
}
.footer-error {
  color: #f87171;
  font-weight: 500;
  flex: 1;
  text-align: right;
}
.save-btn {
  color: rgba(255, 255, 255, 0.5);
  background: rgba(212, 160, 72, 0.12);
  padding: 4px 12px;
}
.save-btn:hover:not(:disabled) {
  color: rgba(255, 255, 255, 0.8);
  background: rgba(212, 160, 72, 0.22);
}
.save-btn:disabled {
  opacity: 0.35;
  cursor: default;
}
</style>
