<script setup lang="ts">
import { ref, computed, nextTick, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useRouter } from "vue-router";
import { burstParticles } from "../utils/burstParticles";
import {
  appConfig,
  personaStore,
  skillsLiteStore,
  savePersonas,
  saveSkillsLites,
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
  Sparkles,
  Globe,
  GlobeOff,
  Check,
} from "@lucide/vue";
import { useI18n } from "vue-i18n";
import { presetMeta } from "../services/websearch";
import { capHeight, chevronTransform } from "../shared/dropdown";

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

// ── Skills Lite selector (skills_lite mode) ──
const activeSkillsLiteName = computed(() => {
  const s = skillsLiteStore.skillsLites.find((s) => s.enabled);
  return s?.name || null;
});

const showSkillsLiteDropdown = ref(false);
const skillsLiteDropdownRef = ref<HTMLDivElement | null>(null);
const skillsLiteBtnRef = ref<HTMLButtonElement | null>(null);
const skillsLiteMenuRef = ref<HTMLDivElement | null>(null);
const skillsLiteDropdownPos = ref({ top: 0, left: 0 });

// ── Dropdown positioning (shared by persona / skills-lite / language) ──
// Places the menu at the trigger button's bottom-left, then flips it above
// the button when there isn't enough room below. `anchorLeft` is the wrapper
// left (persona/skills-lite align to the wrapper edge) or the button left
// (language aligns to the button itself).
function openDropdown(
  btn: HTMLButtonElement,
  menu: HTMLDivElement | null,
  pos: typeof personaDropdownPos,
  anchorLeft: number,
) {
  const rect = btn.getBoundingClientRect();
  pos.value = { top: rect.bottom + 4, left: anchorLeft };
  nextTick(() => {
    if (!menu) return;
    const menuH = menu.offsetHeight;
    const spaceBelow = window.innerHeight - rect.bottom - 4;
    const spaceAbove = rect.top - 4;
    if (menuH > spaceBelow && menuH <= spaceAbove) {
      pos.value = { top: rect.top - menuH - 4, left: anchorLeft };
    }
  });
}

function toggleSkillsLiteDropdown() {
  if (!showSkillsLiteDropdown.value && skillsLiteBtnRef.value) {
    const anchorLeft = skillsLiteDropdownRef.value?.getBoundingClientRect().left ?? skillsLiteBtnRef.value.getBoundingClientRect().left;
    showSkillsLiteDropdown.value = true;
    nextTick(() => openDropdown(skillsLiteBtnRef.value!, skillsLiteMenuRef.value, skillsLiteDropdownPos, anchorLeft));
  } else {
    showSkillsLiteDropdown.value = false;
  }
}

function selectSkillsLite(index: number) {
  for (const s of skillsLiteStore.skillsLites) s.enabled = false;
  skillsLiteStore.skillsLites[index].enabled = true;
  showSkillsLiteDropdown.value = false;
  saveSkillsLites();
  emit("result-stale");
}

const skillsLiteDropdownStyle = computed(() => capHeight(skillsLiteStore.skillsLites.length));

// ── Empty-state hint modal ──
const emptyHintTarget = ref<'persona' | 'dict' | 'websearch' | null>(null);

// i18n keys for the empty-state overlay title/body, keyed by target.
const emptyHintKeys: Record<'persona' | 'dict' | 'websearch', { title: string; body: string }> = {
  persona: { title: 'floating.emptyPersonaTitle', body: 'floating.emptyPersonaBody' },
  dict: { title: 'floating.emptyDictTitle', body: 'floating.emptyDictBody' },
  websearch: { title: 'floating.emptyWebSearchTitle', body: 'floating.emptyWebSearchBody' },
};

// ── Web search provider toggle (persona-style: on/off + chevron dropdown) ──
const webSearchOn = computed(() => appConfig.web_search_providers.some(p => p.enabled));
const activeWebSearchProviderName = computed(() => {
  const p = appConfig.web_search_providers.find(p => p.enabled);
  return p ? (p.custom_name || presetMeta(p.preset).label) : null;
});

const showWebSearchDropdown = ref(false);
const webSearchDropdownRef = ref<HTMLDivElement | null>(null);
const webSearchBtnRef = ref<HTMLButtonElement | null>(null);
const webSearchMenuRef = ref<HTMLDivElement | null>(null);
const webSearchDropdownPos = ref({ top: 0, left: 0 });
const webSearchDropdownStyle = computed(() => capHeight(appConfig.web_search_providers.length));

function toggleWebSearchProvider(e: MouseEvent) {
  if (webSearchOn.value) {
    for (const p of appConfig.web_search_providers) p.enabled = false;
    appConfig.web_search_active_index = -1;
  } else {
    for (let i = 0; i < appConfig.web_search_providers.length; i++) {
      const meta = presetMeta(appConfig.web_search_providers[i].preset);
      if (!meta.keyRequired || appConfig.web_search_providers[i].api_key) {
        appConfig.web_search_providers[i].enabled = true;
        appConfig.web_search_active_index = i;
        burstParticles(e.currentTarget as HTMLElement);
        break;
      }
    }
  }
  appConfig.web_search_enabled_in_skills_lite = webSearchOn.value;
  emit("result-stale");
}

function toggleWebSearchDropdown() {
  if (showWebSearchDropdown.value) {
    showWebSearchDropdown.value = false;
    return;
  }
  const anchorLeft = webSearchDropdownRef.value?.getBoundingClientRect().left ?? 0;
  showWebSearchDropdown.value = true;
  nextTick(() => openDropdown(webSearchBtnRef.value!, webSearchMenuRef.value, webSearchDropdownPos, anchorLeft));
}

function selectWebSearchProvider(index: number) {
  for (const p of appConfig.web_search_providers) p.enabled = false;
  appConfig.web_search_providers[index].enabled = true;
  appConfig.web_search_active_index = index;
  appConfig.web_search_enabled_in_skills_lite = true;
  showWebSearchDropdown.value = false;
  emit("result-stale");
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
    const anchorLeft = personaDropdownRef.value?.getBoundingClientRect().left ?? personaBtnRef.value.getBoundingClientRect().left;
    showPersonaDropdown.value = true;
    nextTick(() => openDropdown(personaBtnRef.value!, personaMenuRef.value, personaDropdownPos, anchorLeft));
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
    const anchorLeft = langBtnRef.value.getBoundingClientRect().left;
    showLangDropdown.value = true;
    nextTick(() => openDropdown(langBtnRef.value!, langMenuRef.value, langDropdownPos, anchorLeft));
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
const personaDropdownStyle = computed(() => capHeight(personaStore.personas.length));
const langDropdownStyle = computed(() => capHeight(targetLanguages.value.length));

function closeAllDropdowns() {
  showPersonaDropdown.value = false;
  showLangDropdown.value = false;
  showSkillsLiteDropdown.value = false;
  showWebSearchDropdown.value = false;
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

  if (
    skillsLiteDropdownRef.value?.contains(target) ||
    skillsLiteMenuRef.value?.contains(target)
  ) {
    return;
  }
  showSkillsLiteDropdown.value = false;

  if (
    webSearchDropdownRef.value?.contains(target) ||
    webSearchMenuRef.value?.contains(target)
  ) {
    return;
  }
  showWebSearchDropdown.value = false;
}

async function handleEmptyHintGo() {
  const target = emptyHintTarget.value;
  emptyHintTarget.value = null;
  if (target === 'dict') {
    await invoke("open_settings_window");
    router.push('/settings/dictionary');
  } else if (target === 'persona') {
    await invoke("open_settings_window");
    router.push('/settings?tab=translation&scrollTo=persona');
  } else if (target === 'websearch') {
    await invoke("open_settings_window");
    router.push('/settings?scrollTo=websearch');
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
  <!-- Skills Lite mode: only skills-lite selector -->
  <template v-if="appConfig.active_mode === 'skills_lite'">
    <div class="skills-lite-wrap" ref="skillsLiteDropdownRef">
      <button
        ref="skillsLiteBtnRef"
        @click="toggleSkillsLiteDropdown"
        class="skills-lite-btn"
        :class="{ active: showSkillsLiteDropdown }"
        :title="t('floating.selectSkillsLite')"
      >
        <Sparkles :size="11" :stroke-width="1.8" />
        <span v-if="activeSkillsLiteName" class="truncate max-w-[5em] min-w-0">{{ activeSkillsLiteName }}</span>
        <ChevronDown :size="10" :stroke-width="2" class="toolbar-chevron"
          :style="{ transform: chevronTransform(showSkillsLiteDropdown, props.growAbove) }" />
      </button>

      <Teleport to="body">
        <Transition name="dropdown">
          <div
            v-if="showSkillsLiteDropdown && skillsLiteStore.skillsLites.length > 0"
            ref="skillsLiteMenuRef"
            class="model-dropdown skills-lite-dropdown"
            :style="{ top: skillsLiteDropdownPos.top + 'px', left: skillsLiteDropdownPos.left + 'px', ...skillsLiteDropdownStyle }"
          >
            <button
              v-for="(skillsLite, si) in skillsLiteStore.skillsLites"
              :key="si"
              @click="selectSkillsLite(si)"
              class="model-option"
              :class="{ selected: skillsLite.enabled }"
            >
              <span class="truncate">{{ skillsLite.name }}</span>
              <span v-if="skillsLite.enabled" class="check-mark"><Check :size="10" :stroke-width="2.5" /></span>
            </button>
          </div>
        </Transition>
      </Teleport>
    </div>

    <!-- Web search provider toggle + selector (skills_lite mode only).
         Mirroring the persona pattern: ghost when empty, on/off toggle
         with a chevron dropdown to pick the active provider otherwise. -->
    <div
      v-if="appConfig.web_search_providers.length > 0"
      class="persona-wrap"
      :class="{ on: webSearchOn }"
      ref="webSearchDropdownRef"
    >
      <button
        @click="toggleWebSearchProvider($event)"
        class="persona-toggle"
        :class="{ on: webSearchOn }"
        :title="webSearchOn ? t('floating.webSearchOn') : t('floating.webSearchOff')"
      >
        <Globe v-if="webSearchOn" :size="11" :stroke-width="1.8" />
        <GlobeOff v-else :size="11" :stroke-width="1.8" />
        <span v-if="webSearchOn" class="status-dot on" />
        <span class="truncate max-w-[3em] min-w-0">{{ webSearchOn ? activeWebSearchProviderName : '' }}</span>
      </button>
      <button
        ref="webSearchBtnRef"
        @click="toggleWebSearchDropdown"
        class="persona-chevron"
        :class="{ on: webSearchOn, active: showWebSearchDropdown }"
      >
        <ChevronDown
          :size="10" :stroke-width="2" class="toolbar-chevron"
          :style="{ transform: chevronTransform(showWebSearchDropdown, props.growAbove) }"
        />
      </button>
      <Teleport to="body">
        <Transition name="dropdown">
          <div
            v-if="showWebSearchDropdown"
            ref="webSearchMenuRef"
            class="model-dropdown"
            :style="{ top: webSearchDropdownPos.top + 'px', left: webSearchDropdownPos.left + 'px', ...webSearchDropdownStyle }"
          >
            <button
              v-for="(provider, pi) in appConfig.web_search_providers"
              :key="pi"
              @click="selectWebSearchProvider(pi)"
              class="model-option"
              :class="{ selected: provider.enabled }"
            >
              <span class="flex items-center gap-2 min-w-0">
                <component :is="presetMeta(provider.preset).icon" :size="14" :stroke-width="1.8" class="shrink-0" />
                <span class="truncate">{{ provider.custom_name || presetMeta(provider.preset).label }}</span>
              </span>
              <span v-if="provider.enabled" class="check-mark"><Check :size="10" :stroke-width="2.5" /></span>
            </button>
          </div>
        </Transition>
      </Teleport>
    </div>
    <button
      v-else
      class="ghost-btn"
      @click="emptyHintTarget = 'websearch'"
      :title="t('floating.noWebSearchProvider')"
    >
      <GlobeOff :size="11" :stroke-width="1.8" />
    </button>
  </template>

  <!-- Translate mode: language + persona + dict -->
  <template v-else>
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
        :style="{ transform: chevronTransform(showLangDropdown, props.growAbove) }" />
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
            <span v-if="appConfig.target_lang === lang" class="check-mark"><Check :size="10" :stroke-width="2.5" /></span>
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
      <span v-if="personaOn" class="status-dot on" />
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
        :style="{ transform: chevronTransform(showPersonaDropdown, props.growAbove) }" />
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
            <span v-if="persona.enabled" class="check-mark"><Check :size="10" :stroke-width="2.5" /></span>
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
    <span v-if="appConfig.user_dict_enabled" class="status-dot on" />
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
  </template>

  <!-- Empty-state hint overlay -->
  <Teleport to="body">
    <Transition name="fade">
      <div
        v-if="emptyHintTarget"
        class="empty-hint-overlay"
      >
        <p class="empty-hint-title">
          {{ emptyHintTarget ? t(emptyHintKeys[emptyHintTarget].title) : '' }}
        </p>
        <p class="empty-hint-body">
          {{ emptyHintTarget ? t(emptyHintKeys[emptyHintTarget].body) : '' }}
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
  gap: var(--space-1);
  height: 28px;
  padding: 0 var(--space-2) 0 7px;
  border-radius: var(--radius-md);
  font-size: var(--text-xs);
  font-weight: var(--weight-medium);
  color: var(--color-text-muted);
  background: var(--color-surface);
  border: 1px solid var(--color-surface);
  transition: color 0.15s ease, background 0.15s ease, border-color 0.15s ease, transform 0.15s ease;
}
.lang-btn:hover,
.lang-btn.active {
  color: var(--color-text-secondary);
  background: var(--color-surface-hover);
  border-color: var(--color-border-hover);
}
.lang-btn:active { transform: translateY(0.5px); }
.lang-btn:focus-visible { outline: 2px solid var(--color-accent-border); outline-offset: 1px; }
.lang-btn:disabled { opacity: .4; cursor: not-allowed; }

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
  padding: 0 var(--space-1) 0 var(--space-2);
  border-radius: var(--radius-md) 0 0 var(--radius-md);
  font-size: var(--text-xs);
  font-weight: var(--weight-medium);
  color: var(--color-text-muted);
  background: var(--color-surface);
  border: 1px solid var(--color-surface);
  border-right: none;
  transition: color 0.15s ease, background 0.15s ease, border-color 0.15s ease;
}
.persona-toggle.on { padding-right: 10px; }
.persona-toggle:hover {
  color: var(--color-text-secondary);
  background: var(--color-border);
}
.persona-toggle:focus-visible { outline: 2px solid var(--color-accent-border); outline-offset: 1px; }
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
.status-dot {
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
  border-radius: 0 var(--radius-md) var(--radius-md) 0;
  color: var(--color-text-muted);
  background: var(--color-surface);
  border: 1px solid var(--color-surface);
  border-left: 1px solid var(--color-surface);
  transition: color 0.15s ease, background 0.15s ease, border-color 0.15s ease;
}
.persona-chevron:hover,
.persona-chevron.active {
  color: var(--color-text-secondary);
  background: var(--color-border);
  border-color: var(--color-border);
}
.persona-chevron:active { background: var(--color-border-hover); }
.persona-chevron:focus-visible { outline: 2px solid var(--color-accent-border); outline-offset: 1px; }
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
  border-radius: var(--radius-md);
  border: 1px solid var(--color-surface);
  background: var(--color-surface);
  color: var(--color-text-muted);
  cursor: pointer;
  transition: color 0.15s ease, background 0.15s ease, border-color 0.15s ease;
  font-size: var(--text-xs);
  font-weight: var(--weight-medium);
  font-family: inherit;
  flex-shrink: 0;
}
.dict-toggle:hover {
  color: var(--color-text-secondary);
  background: var(--color-border);
}
.dict-toggle:active { background: var(--color-border-hover); }
.dict-toggle:focus-visible { outline: 2px solid var(--color-accent-border); outline-offset: 1px; }
.dict-toggle:disabled { opacity: .4; cursor: not-allowed; }
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

/* ── Ghost button (empty state) ── */
.ghost-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  height: 28px;
  width: 28px;
  border-radius: var(--radius-md);
  border: 1px dashed var(--color-border-hover);
  background: transparent;
  color: var(--color-text-muted);
  cursor: pointer;
  transition: color 0.15s ease, background 0.15s ease, border-color 0.15s ease, transform 0.15s ease;
  flex-shrink: 0;
}
.ghost-btn:hover {
  border-color: var(--color-text-muted);
  color: var(--color-text-secondary);
  background: var(--color-surface);
}
.ghost-btn:active { transform: scale(0.95); }
.ghost-btn:focus-visible { outline: 2px solid var(--color-accent-border); outline-offset: 1px; }

/* Skills Lite selector */
.skills-lite-wrap { display: inline-flex; flex-shrink: 0; }
.skills-lite-btn {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  height: 28px;
  padding: 0 var(--space-2) 0 7px;
  border-radius: var(--radius-md);
  font-size: var(--text-xs);
  font-weight: var(--weight-medium);
  color: var(--color-text-muted);
  background: var(--color-surface);
  border: 1px solid var(--color-surface);
  transition: color 0.15s ease, background 0.15s ease, border-color 0.15s ease, transform 0.15s ease;
}
.skills-lite-btn:hover,
.skills-lite-btn.active {
  color: var(--color-accent);
  background: var(--color-accent-bg);
  border-color: var(--color-accent-border);
}
.skills-lite-btn:active { transform: translateY(0.5px); }
.skills-lite-btn:focus-visible { outline: 2px solid var(--color-accent-border); outline-offset: 1px; }
.skills-lite-btn:disabled { opacity: .4; cursor: not-allowed; }

/* ── Empty-state hint (full-window overlay) ── */
.empty-hint-overlay {
  position: absolute;
  inset: 0;
  z-index: 50;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--space-5);
  text-align: center;
  background: var(--color-bg);
}
.empty-hint-title {
  font-size: var(--text-md);
  font-weight: var(--weight-semibold);
  line-height: 1.4;
  color: var(--color-text);
  margin-bottom: var(--space-1);
}
.empty-hint-body {
  font-size: var(--text-base);
  line-height: 1.55;
  color: var(--color-text-secondary);
  margin-bottom: var(--space-4);
}
.empty-hint-actions {
  display: flex;
  gap: var(--space-2);
  justify-content: center;
}
.empty-hint-cancel {
  height: 32px;
  padding: 0 var(--space-4);
  border-radius: var(--radius-md);
  font-size: var(--text-base);
  font-weight: var(--weight-medium);
  color: var(--color-text-secondary);
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  cursor: pointer;
  transition: color 0.15s ease, background 0.15s ease, border-color 0.15s ease, transform 0.15s ease;
}
.empty-hint-cancel:hover {
  color: var(--color-text);
  background: var(--color-surface-hover);
  border-color: var(--color-border-hover);
}
.empty-hint-cancel:active { transform: translateY(0.5px); }
.empty-hint-cancel:focus-visible { outline: 2px solid var(--color-accent-border); outline-offset: 1px; }
.empty-hint-go {
  height: 32px;
  padding: 0 var(--space-4);
  border-radius: var(--radius-md);
  font-size: var(--text-base);
  font-weight: var(--weight-medium);
  color: var(--color-on-accent);
  background: var(--color-accent);
  border: none;
  cursor: pointer;
  transition: background 0.15s ease, transform 0.15s ease;
}
.empty-hint-go:hover { background: color-mix(in srgb, var(--color-accent) 88%, var(--color-bg)); }
.empty-hint-go:active { transform: translateY(0.5px); }
.empty-hint-go:focus-visible { outline: 2px solid var(--color-accent-border); outline-offset: 1px; }

@media (prefers-reduced-motion: reduce) {
  .persona-wrap.on,
  .dict-toggle.on { animation: none; }
  .lang-btn:active,
  .skills-lite-btn:active,
  .ghost-btn:active,
  .empty-hint-cancel:active,
  .empty-hint-go:active { transform: none; }
}
</style>
