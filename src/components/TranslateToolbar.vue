<script setup lang="ts">
import { ref, computed, nextTick, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useRouter } from "vue-router";
import {
  appConfig,
  personaStore,
  savePersonas,
  getOrderedLanguages,
  dictStore,
  refreshDictStatus,
} from "../stores/config";
import { getLangName, getLangCode } from "../constants/languages";
import {
  Languages,
  ChevronDown,
  UserCircle,
  BookText,
} from "@lucide/vue";
import { useI18n } from "vue-i18n";

const { t } = useI18n();
const router = useRouter();

const props = defineProps<{
  growAbove: boolean;
}>();

const emit = defineEmits<{
  "result-stale": [];
}>();

// ── Persona selector ──
const lastActivePersonaIndex = ref(0);
const activePersonaName = computed(() => {
  const p = personaStore.personas.find((p) => p.enabled);
  return p?.name || null;
});
const personaOn = computed(() => personaStore.personas.some((p) => p.enabled));
const displayPersonaName = computed(() => {
  if (activePersonaName.value) return activePersonaName.value;
  const i = lastActivePersonaIndex.value;
  return i < personaStore.personas.length ? personaStore.personas[i].name : (personaStore.personas[0]?.name || 'Persona');
});

const showPersonaDropdown = ref(false);
const personaDropdownRef = ref<HTMLDivElement | null>(null);
const personaBtnRef = ref<HTMLButtonElement | null>(null);
const personaMenuRef = ref<HTMLDivElement | null>(null);
const personaDropdownPos = ref({ top: 0, left: 0 });

// ── Empty-state hint modal ──
const emptyHintTarget = ref<'persona' | 'dict' | null>(null);

function burstParticles(el: HTMLElement) {
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  const rect = el.getBoundingClientRect();
  const cx = rect.left + rect.width / 2;
  const cy = rect.top + rect.height / 2;
  const count = 7;
  for (let i = 0; i < count; i++) {
    const p = document.createElement('span');
    p.className = 'toggle-burst-particle';
    const angle = (Math.PI * 2 / count) * i + (Math.random() - 0.5) * 0.8;
    const dist = 12 + Math.random() * 14;
    const size = 2.5 + Math.random() * 2;
    p.style.setProperty('--tx', `${Math.cos(angle) * dist}px`);
    p.style.setProperty('--ty', `${Math.sin(angle) * dist}px`);
    p.style.width = `${size}px`;
    p.style.height = `${size}px`;
    p.style.left = `${cx}px`;
    p.style.top = `${cy}px`;
    p.style.animationDelay = `${Math.random() * 40}ms`;
    document.body.appendChild(p);
    p.addEventListener('animationend', () => p.remove());
  }
}

function togglePersona(e: MouseEvent) {
  const wasOn = personaStore.personas.some((p) => p.enabled);
  const active = personaStore.personas.findIndex((p) => p.enabled);
  if (active >= 0) {
    personaStore.personas[active].enabled = false;
  } else {
    const i = lastActivePersonaIndex.value < personaStore.personas.length
      ? lastActivePersonaIndex.value : 0;
    personaStore.personas[i].enabled = true;
  }
  const nowOn = !wasOn;
  if (nowOn) burstParticles(e.currentTarget as HTMLElement);
  savePersonas();
  emit("result-stale");
}

function toggleDict(e: MouseEvent) {
  const turning = !appConfig.user_dict_enabled;
  appConfig.user_dict_enabled = turning;
  if (turning) burstParticles(e.currentTarget as HTMLElement);
  emit("result-stale");
}

function togglePersonaDropdown() {
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
  for (const p of personaStore.personas) p.enabled = false;
  personaStore.personas[index].enabled = true;
  lastActivePersonaIndex.value = index;
  showPersonaDropdown.value = false;
  savePersonas();
  emit("result-stale");
}

// ── Language selector ──
const langCode = computed(() => getLangCode(appConfig.target_lang));
const showLangDropdown = ref(false);
const langDropdownRef = ref<HTMLDivElement | null>(null);
const langBtnRef = ref<HTMLButtonElement | null>(null);
const langMenuRef = ref<HTMLDivElement | null>(null);
const langDropdownPos = ref({ top: 0, left: 0 });
const targetLanguages = computed(() => getOrderedLanguages());

function toggleLangDropdown() {
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
  refreshDictStatus();
  emit("result-stale");
}

// ── Dropdown max-height (2 items visible, scroll beyond) ──
const ITEM_H = 28;
const PAD = 6;
const capHeight = (n: number) => n > 2 ? { maxHeight: `${2 * ITEM_H + PAD}px` } : {};
const personaDropdownStyle = computed(() => capHeight(personaStore.personas.length));
const langDropdownStyle = computed(() => capHeight(targetLanguages.value.length));

function closeAllDropdowns() {
  showPersonaDropdown.value = false;
  showLangDropdown.value = false;
}

function onDocumentClick(e: MouseEvent) {
  const target = e.target as Node;
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

const chevronTransform = (open: boolean) =>
  `rotate(${open === props.growAbove ? 0 : 180}deg)`;

async function handleEmptyHintGo() {
  const target = emptyHintTarget.value;
  emptyHintTarget.value = null;
  if (target === 'dict') {
    await invoke("open_settings_window");
    router.push('/settings/dictionary');
  } else if (target === 'persona') {
    await invoke("open_settings_window");
    router.push('/settings?tab=translation&scrollTo=persona');
  }
}

onMounted(() => {
  document.addEventListener("mousedown", onDocumentClick);
});

onUnmounted(() => {
  document.removeEventListener("mousedown", onDocumentClick);
});

defineExpose({ closeAllDropdowns });
</script>

<template>
  <!-- Language selector -->
  <div class="lang-wrap" ref="langDropdownRef">
    <button
      ref="langBtnRef"
      @click="toggleLangDropdown"
      class="lang-btn"
      :class="{ active: showLangDropdown }"
      :title="t('floating.targetLanguage')"
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
            <span class="truncate">{{ getLangName(lang) }}</span>
            <span v-if="appConfig.target_lang === lang" class="check-mark">&#10003;</span>
          </button>
        </div>
      </Transition>
    </Teleport>
  </div>

  <!-- Persona toggle + selector -->
  <div v-if="personaStore.personas.length > 0" class="persona-wrap" :class="{ on: personaOn }" ref="personaDropdownRef">
    <button
      @click="togglePersona($event)"
      class="persona-toggle"
      :class="{ on: personaOn }"
      :title="personaOn ? t('floating.disablePersona') : t('floating.enablePersona')"
    >
      <UserCircle :size="11" :stroke-width="1.8" />
      <span v-if="personaOn" class="persona-dot on" />
      <span class="truncate max-w-[3em] min-w-0">{{ personaOn ? displayPersonaName : '' }}</span>
    </button>
    <button
      v-if="personaStore.personas.length > 0"
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
            v-for="(persona, pi) in personaStore.personas"
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

  <!-- Persona ghost (empty state) -->
  <button
    v-if="personaStore.personas.length === 0"
    class="ghost-btn"
    @click="emptyHintTarget = 'persona'"
    :title="t('floating.noPersonasAvailable')"
  >
    <UserCircle :size="11" :stroke-width="1.8" />
  </button>

  <!-- Dictionary toggle -->
  <button
    v-if="dictStore.hasEntries"
    @click="toggleDict($event)"
    class="dict-toggle"
    :class="{ on: appConfig.user_dict_enabled }"
    :title="appConfig.user_dict_enabled ? t('floating.disableDict') : t('floating.enableDict')"
  >
    <BookText :size="11" :stroke-width="1.8" />
    <span v-if="appConfig.user_dict_enabled" class="dict-dot on" />
  </button>

  <!-- Dictionary ghost (empty state) -->
  <button
    v-if="!dictStore.hasEntries"
    class="ghost-btn"
    @click="emptyHintTarget = 'dict'"
    :title="t('floating.noDictAvailable')"
  >
    <BookText :size="11" :stroke-width="1.8" />
  </button>

  <!-- Empty-state hint overlay -->
  <Teleport to="body">
    <Transition name="fade">
      <div
        v-if="emptyHintTarget"
        class="empty-hint-overlay"
      >
        <p class="empty-hint-title">
          {{ emptyHintTarget === 'persona' ? t('floating.emptyPersonaTitle') : t('floating.emptyDictTitle') }}
        </p>
        <p class="empty-hint-body">
          {{ emptyHintTarget === 'persona' ? t('floating.emptyPersonaBody') : t('floating.emptyDictBody') }}
        </p>
        <div class="empty-hint-actions">
          <button
            class="empty-hint-cancel"
            @click="emptyHintTarget = null"
          >
            {{ t('common.cancel') }}
          </button>
          <button
            class="empty-hint-go"
            @click="handleEmptyHintGo"
          >
            {{ t('floating.goToSettings') }}
          </button>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
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
  color: var(--color-text-muted);
  background: var(--color-surface);
  border: 1px solid var(--color-surface);
  transition: all 0.15s ease;
}
.lang-btn:hover,
.lang-btn.active {
  color: var(--color-text-secondary);
  background: var(--color-surface-hover);
  border-color: var(--color-border-hover);
}
.toolbar-chevron {
  color: var(--color-text-muted);
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
.persona-wrap.on {
  animation: toggle-pop 0.35s cubic-bezier(0.2, 0.8, 0.3, 1);
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
  color: var(--color-text-muted);
  background: var(--color-surface);
  border: 1px solid var(--color-surface);
  border-right: none;
  transition: all 0.18s ease;
}
.persona-toggle.on { padding-right: 10px; }
.persona-toggle:hover {
  color: var(--color-text-secondary);
  background: var(--color-border);
}
.persona-toggle.on {
  color: var(--color-accent);
  background: var(--color-accent-bg);
  border-color: var(--color-accent-border);
}
.persona-toggle.on:hover {
  color: var(--color-accent);
  background: var(--color-accent-bg);
}

/* Status dot */
.persona-dot {
  width: 5px; height: 5px; border-radius: 50%;
  background: var(--color-accent);
  box-shadow: 0 0 5px var(--color-accent-border);
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
  color: var(--color-text-muted);
  background: var(--color-surface);
  border: 1px solid var(--color-surface);
  border-left: 1px solid var(--color-surface);
  transition: all 0.15s ease;
}
.persona-chevron:hover,
.persona-chevron.active {
  color: var(--color-text-secondary);
  background: var(--color-border);
  border-color: var(--color-border);
}
.persona-chevron.on {
  border-color: var(--color-accent-border);
  background: var(--color-accent-bg);
}
.persona-chevron.on:hover,
.persona-chevron.on.active {
  border-color: var(--color-accent-border);
  background: var(--color-accent-bg);
  color: var(--color-accent);
}

/* ── Dictionary toggle ── */
.dict-toggle {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 28px;
  padding: 0 10px;
  border-radius: 8px;
  border: 1px solid var(--color-surface);
  background: var(--color-surface);
  color: var(--color-text-muted);
  cursor: pointer;
  transition: all 0.18s ease;
  font-size: 11px;
  font-family: inherit;
  flex-shrink: 0;
}
.dict-toggle:hover {
  color: var(--color-text-secondary);
  background: var(--color-border);
}
.dict-toggle.on {
  color: var(--color-accent);
  background: var(--color-accent-bg);
  border-color: var(--color-accent-border);
  animation: toggle-pop 0.35s cubic-bezier(0.2, 0.8, 0.3, 1);
}
.dict-toggle.on:hover {
  color: var(--color-accent);
  background: var(--color-accent-bg);
}
.dict-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: var(--color-accent);
  box-shadow: 0 0 5px var(--color-accent-border);
  flex-shrink: 0;
}

/* ── Ghost button (empty state) ── */
.ghost-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  height: 28px;
  width: 28px;
  border-radius: 8px;
  border: 1px dashed var(--color-border-hover);
  background: transparent;
  color: var(--color-text-muted);
  cursor: pointer;
  transition: all 0.15s ease;
  flex-shrink: 0;
}
.ghost-btn:hover {
  border-color: var(--color-text-muted);
  color: var(--color-text-secondary);
  background: var(--color-surface);
}

/* Model dropdown (shared base for lang/persona dropdowns) */
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

/* Transitions */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.15s ease-out;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

/* ── Empty-state hint (full-window overlay) ── */
.empty-hint-overlay {
  position: absolute;
  inset: 0;
  z-index: 50;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 20px;
  text-align: center;
  background: var(--color-bg);
}
.empty-hint-title {
  font-size: 13px;
  font-weight: 600;
  line-height: 1.4;
  color: var(--color-text);
  margin-bottom: 6px;
}
.empty-hint-body {
  font-size: 12px;
  line-height: 1.55;
  color: var(--color-text-secondary);
  margin-bottom: 16px;
}
.empty-hint-actions {
  display: flex;
  gap: 8px;
  justify-content: center;
}
.empty-hint-cancel {
  height: 32px;
  padding: 0 16px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 500;
  color: var(--color-text-secondary);
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  cursor: pointer;
  transition: all 0.15s ease;
}
.empty-hint-cancel:hover {
  color: var(--color-text);
  background: var(--color-surface-hover);
  border-color: var(--color-border-hover);
}
.empty-hint-go {
  height: 32px;
  padding: 0 16px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 500;
  color: white;
  background: var(--color-accent);
  border: none;
  cursor: pointer;
  transition: all 0.15s ease;
}
.empty-hint-go:hover {
  filter: brightness(1.1);
}

@media (prefers-reduced-motion: reduce) {
  .persona-wrap.on,
  .dict-toggle.on { animation: none; }
}
</style>
