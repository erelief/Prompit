<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  appConfig,
  loadProviderPresets,
  saveConfig as persistConfig,
  loadConfig,
} from "../stores/config";
import { getTheme, setTheme } from "../composables/useTheme";
import type { ProviderConfig, ProviderPreset, PresetVariantEndpoint, PresetVariantRegion, WebSearchProviderConfig } from "../stores/config";
import {
  resolvePreset,
  presetBelongsToFamily,
  defaultSelection,
  endpointsOf,
  defaultEndpoint,
  regionLabel,
  endpointLabel,
  variantRegionLabelKey,
  variantEndpointLabelKey,
  isLocalProvider,
} from "../stores/config";
import ProviderIcon from "../components/icons/providers/ProviderIcon.vue";
import ModelCapabilityIcon from "../components/ModelCapabilityIcon.vue";
import DataCategorySelector from "../components/DataCategorySelector.vue";
import { ALL_CATEGORIES, type DataCategory } from "../composables/useDataCategories";
import type { ModelInputCapabilities } from "../stores/config";
import {
  testProviderConnection,
  fetchProviderModels,
} from "../services/llm-client";
import type { FetchModelEntry } from "../services/llm-client";
import {
  ChevronRight,
  ChevronLeft,
  ChevronDown,
  Loader2,
  Check,
  Eye,
  EyeOff,
  Zap,
  PartyPopper,
  PiggyBank,
  Link2,
  X,
  Sun,
  Moon,
  SunMoon,
  Plus,
  Globe,
  Info,
  Upload,
  FolderOpen,
  FileText,
  ShieldAlert,
} from "@lucide/vue";
import { WEB_SEARCH_PRESETS, testWebSearchProvider, presetMeta } from "../services/websearch";
import { useDataImport } from "../composables/useDataImport";

const { t } = useI18n();
const router = useRouter();

// ── Step management ──
// Step IDs: 0=welcome, 1=info, 2=add-provider, 3=lightweight, 4=select-models,
// 5=websearch-info, 6=add-search, 7=done, 8=import (branch step),
// 9=import-summary (branch step — analyzes what landed and routes the user).
// `importMode` switches the linear step sequence to the import path:
//   welcome(0) → info(1) → import(8) → import-summary(9) → (done(7) | websearch(5) | add-provider(2))
// Otherwise the full add-provider path runs: 0→1→2→3→4→5→6→7.
// When the machine already has a web-search provider configured, steps 5 and 6
// are skipped on the linear path too (skipWebSearchSteps), so a user who
// imported web-search but not AI providers is not re-asked about web search.
const importMode = ref(false);
const currentStep = ref(0);
const direction = ref<"forward" | "backward">("forward");

// ── Step 0: App language ──
const appLanguageOptions = [
  { value: "en", label: "English" },
  { value: "zh-CN", label: "简体中文" },
];
const showAppLangMenu = ref(false);

function selectAppLang(lang: string) {
  appConfig.app_lang = lang;
  showAppLangMenu.value = false;
}

const currentAppLangLabel = computed(() => {
  return appLanguageOptions.find(o => o.value === appConfig.app_lang)?.label || "English";
});

const themeOptions = computed(() => [
  { value: "light" as const, icon: Sun, label: t("settings.light") },
  { value: "dark" as const, icon: Moon, label: t("settings.dark") },
  { value: "system" as const, icon: SunMoon, label: t("settings.system") },
]);

const currentPresetLabel = computed(() => {
  if (!selectedPreset.value || selectedPreset.value === "Custom") return t('onboarding.custom');
  return selectedPreset.value;
});

// Computed: current preset FAMILY object (resolves endpoint names to their family)
const currentPresetObj = computed(() => {
  if (!selectedPreset.value) return null;
  return resolvePreset(selectedPreset.value, providerPresets.value).preset || null;
});

// Computed: current variant region/endpoint (only set when a variant family is active)
const currentVariantRegion = computed<PresetVariantRegion | undefined>(() => {
  if (!selectedPreset.value) return undefined;
  return resolvePreset(selectedPreset.value, providerPresets.value).region;
});
const currentVariantEndpoint = computed<PresetVariantEndpoint | undefined>(() => {
  if (!selectedPreset.value) return undefined;
  return resolvePreset(selectedPreset.value, providerPresets.value).endpoint;
});
const variantRegions = computed<PresetVariantRegion[]>(() => currentPresetObj.value?.variants?.regions ?? []);
const variantEndpoints = computed<PresetVariantEndpoint[]>(() => currentVariantRegion.value?.endpoints ?? []);

async function testKeyConnection() {
  if ((!providerForm.value.api_key && !isLocalProvider(providerForm.value, providerPresets.value)) || !providerForm.value.base_url) return;
  isTestingKey.value = true;
  testKeyStatus.value = "";
  testKeyError.value = "";
  const result = await testProviderConnection(providerForm.value);
  testKeyStatus.value = result.ok ? "ok" : "fail";
  if (!result.ok) testKeyError.value = result.error || "";
  isTestingKey.value = false;
  setTimeout(() => { testKeyStatus.value = ""; testKeyError.value = ""; }, 3000);
}

// ── Step 2: Provider form ──
const providerForm = ref<ProviderConfig>({
  name: "",
  api_key: "",
  base_url: "",
  models: [],
  temperature: 0.3,
  max_tokens: 1024,
});
const providerPresets = ref<ProviderPreset[]>([]);
const selectedPreset = ref("");
const showApiKey = ref(false);
const showPresetMenu = ref(false);
const isTestingKey = ref(false);
const testKeyStatus = ref<"ok" | "fail" | "">("");
const testKeyError = ref("");
const showCloseConfirm = ref(false);

// ── Step 3: Models ──
const availableModels = ref<FetchModelEntry[]>([]);
const selectedModels = ref(new Map<string, ModelInputCapabilities>());
const fetchError = ref("");
const isConnecting = ref(false);
const isFetching = ref(false);

// ── Step 6: Search API config ──
const searchSelectedPreset = ref("");
const searchShowPresetMenu = ref(false);
const searchApiKey = ref("");
const searchCustomName = ref("");
const searchShowApiKey = ref(false);
const searchTestStatus = ref<"ok" | "fail" | "">("");
const searchTestError = ref("");
const searchIsTesting = ref(false);

// True when the selected preset needs a key the user hasn't supplied. Keyless
// presets (Firecrawl / AnySearch anonymous tier) never need a key. Shared by
// the next-button gate, the save/test guards, and the test-button binding.
const searchKeyMissing = computed(() => {
  if (!searchSelectedPreset.value) return true;
  return presetMeta(searchSelectedPreset.value).keyRequired && !searchApiKey.value.trim();
});

const searchCanNext = computed(() => !!searchSelectedPreset.value && !searchKeyMissing.value);

// ── Computed ──
// ── Import branch ──
// Entered from step 1 (info) when the user picks "import existing settings"
// instead of adding a provider manually. Uses the shared useDataImport
// composable so the import logic is identical to the Settings page.
const {
  importPath, importPassword, importShowPw, importConfirming,
  importCountdown, importStatus, importBusy,
  importPreview, importSelected, importAnalyzed, importAnalyzing,
  importFileName, importCanAnalyze, importCanConfirm,
  selectImportFile, analyzeImport, requestImport, confirmImport, stopCountdown, resetImport,
} = useDataImport({
  messages: {
    cancelled: t("settings.importData.import.cancelled"),
    success: t("settings.importData.import.success"),
    error: (message: string) => t("settings.importData.error", { message }),
  },
  // On success: reload config from the just-imported files (no restart — the
  // Master Key is already installed in-process), then advance to the
  // import-summary step (9). That step inspects what landed and routes the
  // user to the right next step (done / websearch / add-provider).
  async onSuccess() {
    try {
      await loadConfig();
      importSucceeded.value = true;
    } catch (err) {
      importStatus.value = {
        kind: "error",
        msg: t("settings.importData.error", { message: String(err) }),
      };
    }
  },
});

// Whether the import has completed with usable data. Gates the Next button on
// the import step (8) and switches its view from the import form to a success
// state. Unlike the previous behavior we no longer reject imports that lack a
// provider — instead the summary step (9) routes the user to add one.
const importSucceeded = ref(false);

// Bridge the Set-based selection from the composable to the string[] the
// DataCategorySelector v-model expects.
const importSelectedArray = computed<string[]>({
  get: () => [...importSelected.value],
  set: (v) => { importSelected.value = new Set(v); },
});
const importAvailableCats = computed<DataCategory[]>(() =>
  importPreview.value
    .map((c) => c.id)
    .filter((id): id is DataCategory =>
      (ALL_CATEGORIES as string[]).includes(id),
    ),
);

// Summary-step derived state: what landed after the import.
const importedProviderCount = computed(() =>
  importPreview.value.find((c) => c.id === "providers")?.count ?? 0,
);
const importedWebSearchCount = computed(() =>
  importPreview.value.find((c) => c.id === "websearch")?.count ?? 0,
);
// After loadConfig, appConfig reflects the merged result. The summary page
// reports the live (post-import) provider/websearch presence.
const hasUsableProviders = computed(() =>
  appConfig.providers.length > 0
  && appConfig.providers.some((p) => p.api_key.trim() !== ""),
);
const hasWebSearch = computed(() => appConfig.web_search_providers.length > 0);

// Steps 5 (websearch-info) and 6 (add-search) are skipped whenever a web-search
// provider is already configured — including when the user just imported one,
// or when the machine already had one before onboarding (rule: an app with web
// search but no AI provider still skips the web-search steps).
const skipWebSearchSteps = computed(() => appConfig.web_search_providers.length > 0);

// Tracks where the user was routed after the import-summary step. Null when
// the user never completed an import (normal add-provider onboarding). Used by
// dotSteps to show only the remaining post-import path rather than the full
// linear sequence from step 0.
const postImportTarget = ref<number | null>(null);

// Route the user out of the import-summary step based on what landed.
function routeAfterImport() {
  importMode.value = false;
  direction.value = "forward";
  if (hasUsableProviders.value) {
    postImportTarget.value = hasWebSearch.value ? 7 : 5;
    currentStep.value = postImportTarget.value;
  } else {
    postImportTarget.value = 2;
    currentStep.value = 2;
  }
}

function enterImportBranch() {
  importMode.value = true;
  postImportTarget.value = null;
  direction.value = "forward";
  currentStep.value = 8;
}

function resetImportBranch() {
  importSucceeded.value = false;
  postImportTarget.value = null;
  resetImport();
}

const canProceed = computed(() => {
  switch (currentStep.value) {
    case 2:
      // Provider must have a name + base URL, and either a key or be a local preset.
      return (
        providerForm.value.name.trim() !== "" &&
        providerForm.value.base_url.trim() !== "" &&
        (isLocalProvider(providerForm.value, providerPresets.value) || providerForm.value.api_key.trim() !== "")
      );
    case 8:
      // Import step: Next is only enabled after a successful import. The actual
      // category selection + countdown-confirm drives the import; Next advances
      // to the summary step (9), which decides where to route the user.
      return importSucceeded.value;
    case 9:
      // Import summary: informational, always reachable. The Next button calls
      // routeAfterImport, not the linear increment.
      return true;
    case 4:
      return selectedModels.value.size > 0;
    case 0: case 1: case 3: case 5: case 6: case 7:
      // Info/optional steps are always reachable.
      return true;
    default:
      return false;
  }
});

const isLastStep = computed(() => currentStep.value === 7);

// Step-dot indicator sequence.
// - Import branch: welcome→info→import→summary (4 dots).
// - Post-import (routed to step 2/5/7): only the remaining steps, so the
//   user sees how many steps are left rather than the already-completed 0..1..8..9.
// - Linear (add-provider) branch: 0..7, but 5 and 6 are dropped when a
//   web-search provider is already configured (skipWebSearchSteps).
const dotSteps = computed(() => {
  if (importMode.value) return [0, 1, 8, 9];

  const tgt = postImportTarget.value;
  if (tgt !== null) {
    // Post-import landing: show only what's left.
    if (tgt === 7) return [7];
    if (tgt === 5) return [5, 6, 7];
    // tgt === 2: add-provider path, with or without web-search.
    return skipWebSearchSteps.value ? [2, 3, 4, 7] : [2, 3, 4, 5, 6, 7];
  }

  // Normal add-provider onboarding from scratch.
  return skipWebSearchSteps.value ? [0, 1, 2, 3, 4, 7] : [0, 1, 2, 3, 4, 5, 6, 7];
});
const currentDotIndex = computed(() =>
  dotSteps.value.indexOf(currentStep.value),
);

const shortcutKey = ref("...");

// ── Navigation ──
function goNext() {
  if (!canProceed.value) return;
  if (currentStep.value === 2) {
    confirmProviderAndAdvance();
    return;
  }
  if (currentStep.value === 7) {
    finishOnboarding();
    return;
  }
  if (currentStep.value === 8) {
    // Import succeeded → advance to the summary step (9), which routes onward.
    direction.value = "forward";
    currentStep.value = 9;
    return;
  }
  if (currentStep.value === 9) {
    routeAfterImport();
    return;
  }
  // Step 6: save search config if user filled it in
  if (currentStep.value === 6) {
    saveSearchConfig();
  }
  // Skip web-search steps (5, 6) on the linear path when already configured.
  if (currentStep.value === 4 && skipWebSearchSteps.value) {
    direction.value = "forward";
    currentStep.value = 7;
    return;
  }
  direction.value = "forward";
  currentStep.value++;
}

function saveSearchConfig() {
  if (!searchSelectedPreset.value || searchKeyMissing.value) return;
  const provider: WebSearchProviderConfig = {
    preset: searchSelectedPreset.value,
    api_key: searchApiKey.value.trim(),
    enabled: true,
    custom_name: searchCustomName.value.trim() || currentSearchPresetLabel.value,
  };
  appConfig.web_search_providers.push(provider);
  appConfig.web_search_active_index = appConfig.web_search_providers.length - 1;
  appConfig.web_search_enabled_in_skills_lite = true;
}

const currentSearchPresetLabel = computed(() => {
  if (!searchSelectedPreset.value) return t('onboarding.selectSearchPreset');
  return WEB_SEARCH_PRESETS.find(p => p.id === searchSelectedPreset.value)?.label || searchSelectedPreset.value;
});

const currentSearchPresetObj = computed(() => {
  if (!searchSelectedPreset.value) return null;
  return WEB_SEARCH_PRESETS.find(p => p.id === searchSelectedPreset.value) || null;
});

function applySearchPreset(id: string) {
  searchSelectedPreset.value = id;
  searchShowPresetMenu.value = false;
  // Auto-fill the name with the preset's label (mirrors Settings' applyWebPreset),
  // unless the user has already typed one.
  if (!searchCustomName.value.trim()) {
    searchCustomName.value = presetMeta(id).label;
  }
}

async function testSearchConnection() {
  if (!searchSelectedPreset.value || searchKeyMissing.value) return;
  searchIsTesting.value = true;
  searchTestStatus.value = "";
  searchTestError.value = "";
  const r = await testWebSearchProvider(searchSelectedPreset.value, searchApiKey.value.trim());
  searchTestStatus.value = r.ok ? "ok" : "fail";
  if (!r.ok) searchTestError.value = r.error || "";
  searchIsTesting.value = false;
  setTimeout(() => { searchTestStatus.value = ""; searchTestError.value = ""; }, 3000);
}

function goPrev() {
  if (currentStep.value === 0) return;
  direction.value = "backward";
  // Import branch: step 8 goes back to step 1 (info/choose).
  if (currentStep.value === 8) {
    importMode.value = false;
    currentStep.value = 1;
    return;
  }
  // Summary step (9) goes back to the import step (8).
  if (currentStep.value === 9) {
    currentStep.value = 8;
    return;
  }
  // Done step in import branch goes back to the summary/import step, not 6.
  if (currentStep.value === 7 && importMode.value) {
    currentStep.value = importSucceeded.value ? 9 : 8;
    return;
  }
  // Skip web-search steps (5, 6) backward when already configured.
  if (currentStep.value === 7 && skipWebSearchSteps.value && !importMode.value) {
    currentStep.value = 4;
    return;
  }
  currentStep.value--;
}

// ── Step 0 logic: language is applied immediately via selectAppLang ──

// ── Step 2 logic ──
function applyPreset(presetName: string) {
  const preset = providerPresets.value.find((p) => p.name === presetName);
  if (!preset) return;
  showPresetMenu.value = false;
  if (presetName === "Custom") {
    selectedPreset.value = presetName;
    providerForm.value.name = "";
    providerForm.value.base_url = "";
    providerForm.value.preset = undefined;
    providerForm.value.api_format = undefined;
    return;
  }
  // Variant family → land on its default region/endpoint selection.
  if (preset.variants) {
    const { endpoint } = defaultSelection(preset);
    applyEndpointFields(preset, endpoint);
  } else {
    applyEndpointFields(preset, undefined);
  }
}

/** Shared field writer for both preset apply and endpoint switching. */
function applyEndpointFields(preset: ProviderPreset, endpoint?: PresetVariantEndpoint) {
  selectedPreset.value = endpoint ? endpoint.provider_name : preset.name;
  providerForm.value.name = endpoint ? endpoint.provider_name : (preset.provider_name ?? preset.name);
  providerForm.value.base_url = endpoint ? endpoint.base_url : (preset.base_url ?? "");
  providerForm.value.preset = endpoint ? endpoint.provider_name : preset.name;
  providerForm.value.api_format = preset.api_format && Object.keys(preset.api_format).length > 0
    ? { ...preset.api_format }
    : undefined;
}

/** Switch region; keep current endpoint key if still valid in the new region,
 *  otherwise fall back to the region's default/first endpoint. */
function applyRegion(regionKey: string) {
  const family = currentPresetObj.value;
  if (!family?.variants) return;
  const eps = endpointsOf(family, regionKey);
  const next = eps.find(e => e.key === currentVariantEndpoint.value?.key)
    ?? defaultEndpoint(family, regionKey);
  if (next) applyEndpointFields(family, next);
}

/** Switch endpoint within the current region. */
function applyEndpointByKey(endpointKey: string) {
  const family = currentPresetObj.value;
  const region = currentVariantRegion.value;
  if (!family?.variants || !region) return;
  const ep = region.endpoints.find(e => e.key === endpointKey);
  if (ep) applyEndpointFields(family, ep);
}

// ── Step 3 flow ──
async function confirmProviderAndAdvance() {
  isConnecting.value = true;
  fetchError.value = "";

  const result = await testProviderConnection(providerForm.value);
  if (!result.ok) {
    fetchError.value = result.error || "Connection failed";
    isConnecting.value = false;
    return;
  }

  isConnecting.value = false;
  const ok = await fetchAndApplyModels();
  if (ok) {
    direction.value = "forward";
    currentStep.value = 3;
  }
}

// Shared by the initial connect and the step-3 retry: fetch the model list and
// populate availableModels, reporting any error. Returns whether it succeeded.
async function fetchAndApplyModels(): Promise<boolean> {
  isFetching.value = true;
  fetchError.value = "";
  const modelsResult = await fetchProviderModels(providerForm.value);
  if (!modelsResult.ok || !modelsResult.models || modelsResult.models.length === 0) {
    fetchError.value = modelsResult.error || t('onboarding.noModelsFound');
    isFetching.value = false;
    return false;
  }
  availableModels.value = modelsResult.models;
  isFetching.value = false;
  return true;
}

// ── Step 3 retry ──
async function retryFetchModels() {
  await fetchAndApplyModels();
}

// ── Step 4 logic ──
function toggleModel(entry: FetchModelEntry) {
  const m = new Map(selectedModels.value);
  m.has(entry.id) ? m.delete(entry.id) : m.set(entry.id, entry.input_capabilities);
  selectedModels.value = m;
}

function selectAll() {
  const m = new Map<string, ModelInputCapabilities>();
  for (const e of availableModels.value) m.set(e.id, e.input_capabilities);
  selectedModels.value = m;
}

function deselectAll() {
  selectedModels.value = new Map();
}

// ── Step 4: manual model entry ──
const manualModelInput = ref("");

function addManualModel() {
  const id = manualModelInput.value.trim();
  if (!id) return;
  // Surface a fake entry in the list so it's visible & selectable like fetched ones.
  if (!availableModels.value.some((e) => e.id === id)) {
    availableModels.value = [...availableModels.value, { id, input_capabilities: {} }];
  }
  const m = new Map(selectedModels.value);
  m.set(id, {});
  selectedModels.value = m;
  manualModelInput.value = "";
}

async function finishOnboarding() {
  if (!importMode.value) {
    // Add-provider branch: commit the just-configured provider.
    providerForm.value.models = [...selectedModels.value].map(([id, caps]) => ({
      id,
      input_capabilities: Object.keys(caps).length > 0 ? caps : undefined,
    }));
    appConfig.providers.push({ ...providerForm.value });
    appConfig.translate_active_provider_index = 0;
    appConfig.translate_active_model_index = 0;
  }
  // Import branch: providers were already loaded by the import onSuccess
  // callback — nothing to push here.

  await persistConfig();
  await invoke("set_onboarding_complete");

  // Hide window — don't show anything after finishing
  await invoke("hide_main_window");

  router.replace("/");
}

// ── Click outside to close dropdowns ──
function onRootClick(e: MouseEvent) {
  const t = e.target as HTMLElement;
  if (!t.closest(".sel-wrap")) {
    showAppLangMenu.value = false;
    showPresetMenu.value = false;
  }
}

// ── Close button ──
function handleClose() {
  if (currentStep.value === 7) {
    finishOnboarding();
  } else {
    showCloseConfirm.value = true;
  }
}

function confirmClose() {
  showCloseConfirm.value = false;
  invoke("hide_main_window");
}

// ── Window move ──
// Frameless window: dragging the background (not interactive controls) moves
// the window. Matches the pattern used by Settings/UserData/etc. The exclusion
// list must cover EVERY interactive element — including <label> rows used for
// model selection (step 4) — otherwise mousedown→startDragging swallows the
// click and the user can't toggle individual models.
async function handleDrag(e: MouseEvent) {
  const target = e.target as HTMLElement;
  if (target.closest("button, input, textarea, a, select, label, .sel-wrap, .import-file-pick, .import-pw-toggle, .import-change-btn, .mini-btn, .import-btn, .manual-model-row")) return;
  await getCurrentWindow().startDragging();
}

// ── Init ──
onMounted(async () => {
  invoke<string>("get_shortcut_label").then(s => { shortcutKey.value = s; }).catch(() => {});
  // Ensure window is properly sized and visible for onboarding
  invoke("show_onboarding_window");
  try {
    providerPresets.value = await loadProviderPresets();
  } catch (err) {
    console.error("Failed to load presets:", err);
  }
});
</script>

<template>
  <div class="flex items-center justify-center h-dvh px-6 select-none" style="background: var(--color-bg)" @mousedown="handleDrag" @click="onRootClick">
    <div class="w-full max-w-[520px] flex flex-col relative" style="height: 100%; max-height: 100dvh; min-height: 0">

      <!-- Header: X button only, non-scrollable -->
      <div class="flex items-center justify-end flex-shrink-0 h-10 pr-1">
        <button
          class="flex items-center justify-center w-8 h-8 rounded-lg transition-colors"
          style="color: var(--color-text-muted)"
          @click="handleClose"
          :title="t('common.hide')"
        >
          <X :size="18" :stroke-width="1.5" />
        </button>
      </div>

      <!-- Close confirmation modal -->
      <Transition name="drop">
        <div v-if="showCloseConfirm" class="fixed inset-0 z-50 flex items-center justify-center" style="background: rgba(0,0,0,0.4); backdrop-filter: blur(4px)">
          <div class="rounded-xl p-6 mx-6 max-w-xs w-full" style="background: var(--color-bg); border: 1px solid var(--color-border)">
            <p class="text-sm mb-5" style="color: var(--color-text); line-height: 1.5">
              {{ t('onboarding.exitConfirm') }}
            </p>
            <div class="flex gap-2 justify-end">
              <button
                class="mini-btn"
                :title="t('common.cancel')"
                @click="showCloseConfirm = false"
              >
                <X :size="12" :stroke-width="2.5" />
              </button>
              <button
                class="mini-btn danger-active"
                :title="t('onboarding.exitAnyway')"
                @click="confirmClose"
              >
                <Check :size="12" :stroke-width="2.5" />
              </button>
            </div>
          </div>
        </div>
      </Transition>

      <!-- Content area with transitions -->
      <div class="onb-content flex-1 relative overflow-y-auto min-h-0">
        <div class="w-full max-w-[520px] min-h-full">
        <Transition :name="direction === 'forward' ? 'slide-left' : 'slide-right'" mode="out-in">

          <!-- Step 0: Welcome -->
          <div v-if="currentStep === 0" key="step0" class="flex flex-col items-center justify-center h-full py-10">
            <h1 class="text-5xl font-light tracking-tight mb-3" style="color: var(--color-text)">
              {{ t('onboarding.hello') }}
            </h1>
            <p class="text-base mb-10" style="color: var(--color-text-secondary)">
              {{ t('onboarding.welcomeTitle') }}
            </p>
            <div class="w-full max-w-xs">
              <label class="block text-xs font-medium mb-2 tracking-wide uppercase" style="color: var(--color-text-muted)">
                {{ t('onboarding.selectLanguage') }}
              </label>
              <div class="sel-wrap" style="position: relative">
                <button class="sel-btn w-full" @click="showAppLangMenu = !showAppLangMenu">
                  <span class="sel-text">{{ currentAppLangLabel }}</span>
                  <ChevronDown :size="11" :stroke-width="2" class="sel-arrow" :class="{ rot: showAppLangMenu }" />
                </button>
                <Transition name="drop">
                  <div v-if="showAppLangMenu" class="sel-menu" style="position: absolute; top: 100%; left: 0; right: 0; margin-top: 4px; z-index: 10">
                    <div class="sel-clip">
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
              </div>
            </div>

            <!-- Theme selector -->
            <div class="w-full max-w-xs mt-6">
              <label class="block text-xs font-medium mb-2 tracking-wide uppercase" style="color: var(--color-text-muted)">
                {{ t('onboarding.selectTheme') }}
              </label>
              <div class="theme-toggle compact">
                <button
                  v-for="opt in themeOptions"
                  :key="opt.value"
                  class="theme-btn"
                  :class="{ on: getTheme() === opt.value }"
                  @click="setTheme(opt.value)"
                >
                  <component :is="opt.icon" :size="13" :stroke-width="1.8" />
                  {{ opt.label }}
                </button>
              </div>
            </div>
          </div>

          <!-- Step 1: Info -->
          <div v-else-if="currentStep === 1" key="step1" class="flex flex-col items-center justify-center h-full py-10 relative">
            <div class="w-12 h-12 rounded-full flex items-center justify-center mb-6" style="background: var(--color-accent-bg)">
              <Zap :size="22" style="color: var(--color-accent)" />
            </div>
            <h2 class="text-xl font-medium mb-3" style="color: var(--color-text)">
              {{ t('onboarding.infoTitle') }}
            </h2>
            <p class="text-sm leading-relaxed text-center max-w-sm" style="color: var(--color-text-secondary)">
              {{ t('onboarding.infoBody') }}
            </p>
            <button
              @click="enterImportBranch"
              class="absolute bottom-0 text-xs pb-2 transition-colors hover:underline"
              style="color: var(--color-text-muted); background: none; border: none; cursor: pointer;"
            >
              {{ t('onboarding.importExistingSettings') }}
            </button>
          </div>

          <!-- Step 2: Add Provider -->
          <div v-else-if="currentStep === 2" key="step2" class="flex flex-col py-6">
            <h2 class="text-lg font-medium mb-6" style="color: var(--color-text)">
              {{ t('onboarding.addProviderTitle') }}
            </h2>

            <!-- Preset selector -->
            <div class="mb-5">
              <label class="block text-xs font-medium mb-1.5 tracking-wide uppercase" style="color: var(--color-text-muted)">
                {{ t('onboarding.preset') }}
              </label>
              <div class="sel-wrap" style="position: relative">
                <button class="sel-btn w-full" @click="showPresetMenu = !showPresetMenu">
                  <span class="flex items-center gap-2 min-w-0 flex-1">
                    <ProviderIcon v-if="currentPresetObj?.icon" :icon="currentPresetObj.icon" :size="14" />
                    <span class="sel-text" :style="{ opacity: selectedPreset ? 1 : 0.5 }">{{ currentPresetLabel }}</span>
                  </span>
                  <ChevronDown :size="11" :stroke-width="2" class="sel-arrow" :class="{ rot: showPresetMenu }" />
                </button>
                <Transition name="drop">
                  <div v-if="showPresetMenu" class="sel-menu" style="position: absolute; top: 100%; left: 0; right: 0; margin-top: 4px; z-index: 10">
                    <div class="sel-clip">
                      <button
                        v-for="p in providerPresets" :key="p.name"
                        class="sel-opt"
                        :class="{ hit: presetBelongsToFamily(selectedPreset || undefined, p) || (!selectedPreset && p.name === 'Custom') }"
                        @click="applyPreset(p.name)"
                      >
                        <div class="opt-left"><ProviderIcon :icon="p.icon" :size="14" />
                        <div class="opt-info">
                          <div class="opt-id-row">
                            <span class="opt-id">{{ p.name === 'Custom' ? t('onboarding.custom') : p.name }}</span>
                            <span v-for="s in p.model_series" :key="s" class="opt-series-tag">{{ s }}</span>
                          </div>
                        </div></div>
                        <Check v-if="presetBelongsToFamily(selectedPreset || undefined, p) || (!selectedPreset && p.name === 'Custom')" :size="13" :stroke-width="2.5" />
                      </button>
                    </div>
                  </div>
                </Transition>
              </div>
            </div>

            <!-- Name -->
            <div class="mb-4">
              <label class="block text-xs font-medium mb-1.5 tracking-wide uppercase" style="color: var(--color-text-muted)">
                {{ t('onboarding.providerName') }}
              </label>
              <input
                v-model="providerForm.name"
                type="text"
                class="w-full h-9 px-3 rounded-lg text-sm outline-none transition-colors select-text"
                style="background: var(--color-surface); color: var(--color-text); border: 1px solid var(--color-border)"
              />
              <p v-if="!selectedPreset || selectedPreset === 'Custom'" class="mt-1.5" style="font-size: 10.5px; color: var(--color-text-muted); line-height: 1.4">
                {{ t('settings.openaiCompatHint') }}
              </p>
            </div>

            <!-- Variant selector (only for multi-variant family presets) -->
            <div v-if="currentPresetObj?.variants" class="mb-4">
              <!-- Region row -->
              <div class="ob-variant-row">
                <span class="ob-variant-label">{{ t(variantRegionLabelKey()) }}</span>
                <div class="ob-variant-btns">
                  <button
                    v-for="r in variantRegions" :key="r.key"
                    class="ob-variant-btn"
                    :class="{ active: currentVariantRegion?.key === r.key }"
                    @click="applyRegion(r.key)"
                  >{{ regionLabel(r) }}</button>
                </div>
              </div>
              <!-- Endpoint row (options depend on the selected region) -->
              <div v-if="variantEndpoints.length > 0" class="ob-variant-row">
                <span class="ob-variant-label">{{ t(variantEndpointLabelKey()) }}</span>
                <div class="ob-variant-btns">
                  <button
                    v-for="ep in variantEndpoints" :key="ep.key"
                    class="ob-variant-btn"
                    :class="{ active: currentVariantEndpoint?.key === ep.key }"
                    @click="applyEndpointByKey(ep.key)"
                  >{{ endpointLabel(ep) }}</button>
                </div>
              </div>
            </div>

            <!-- Get API key / download link (below variants) -->
            <p v-if="selectedPreset && selectedPreset !== 'Custom' && (currentVariantEndpoint?.api_url || currentPresetObj?.api_url)" class="mb-4 -mt-1" style="font-size: 10.5px; line-height: 1.4">
              <a :href="currentVariantEndpoint?.api_url || currentPresetObj?.api_url" target="_blank" rel="noopener noreferrer" style="color: var(--color-accent); text-decoration: underline; text-underline-offset: 2px;">
                {{ isLocalProvider(providerForm, providerPresets) ? t('settings.downloadAt', { name: selectedPreset }) : t('settings.getApiKeyAt', { name: selectedPreset }) }}
              </a>
            </p>

            <!-- Base URL -->
            <div class="mb-4">
              <label class="block text-xs font-medium mb-1.5 tracking-wide uppercase" style="color: var(--color-text-muted)">
                {{ t('onboarding.baseUrl') }}
              </label>
              <input
                v-model="providerForm.base_url"
                type="text"
                class="w-full h-9 px-3 rounded-lg text-sm outline-none transition-colors select-text"
                style="background: var(--color-surface); color: var(--color-text); border: 1px solid var(--color-border)"
                placeholder="https://api.example.com/v1"
              />
            </div>

            <!-- API Key -->
            <div class="mb-4">
              <label class="block text-xs font-medium mb-1.5 tracking-wide uppercase" style="color: var(--color-text-muted)">
                {{ t('onboarding.apiKey') }}
              </label>
              <div class="flex gap-2">
                <div class="relative flex-1">
                  <input
                    v-model="providerForm.api_key"
                    :type="showApiKey ? 'text' : 'password'"
                    class="w-full h-9 pl-3 pr-9 rounded-lg text-sm outline-none transition-colors select-text"
                    style="background: var(--color-surface); color: var(--color-text); border: 1px solid var(--color-border)"
                  />
                  <button
                    @click="showApiKey = !showApiKey"
                    class="absolute right-2 top-1/2 -translate-y-1/2 p-0.5 rounded opacity-40 hover:opacity-80 transition-opacity"
                    style="color: var(--color-text)"
                    tabindex="-1"
                  >
                    <Eye v-if="showApiKey" :size="16" />
                    <EyeOff v-else :size="16" />
                  </button>
                </div>
                <button
                  class="flex items-center justify-center h-9 w-9 rounded-lg transition-colors"
                  :style="{
                    background: testKeyStatus === 'ok' ? 'var(--color-accent-bg)' : testKeyStatus === 'fail' ? 'rgba(239,68,68,0.1)' : 'var(--color-surface)',
                    border: '1px solid var(--color-border)',
                    cursor: ((!providerForm.api_key && !isLocalProvider(providerForm, providerPresets)) || !providerForm.base_url || isTestingKey) ? 'not-allowed' : 'pointer',
                    opacity: ((!providerForm.api_key && !isLocalProvider(providerForm, providerPresets)) || !providerForm.base_url) ? 0.4 : 1,
                  }"
                  :disabled="(!providerForm.api_key && !isLocalProvider(providerForm, providerPresets)) || !providerForm.base_url || isTestingKey"
                  @click="testKeyConnection"
                  :title="t('settings.testConnection')"
                >
                  <Loader2 v-if="isTestingKey" :size="14" class="spin" style="color: var(--color-accent)" />
                  <Check v-else-if="testKeyStatus === 'ok'" :size="14" style="color: var(--color-accent)" />
                  <Link2 v-else :size="14" style="color: var(--color-text-muted)" />
                </button>
              </div>
              <p v-if="isTestingKey || testKeyStatus === 'fail' || isConnecting || isFetching || fetchError" class="text-xs mt-1.5 flex items-center gap-1.5" :style="{ color: (testKeyStatus === 'fail' || fetchError) ? 'var(--color-danger)' : 'var(--color-text-secondary)' }">
                <Loader2 v-if="isTestingKey || isConnecting || isFetching" :size="14" class="spin" style="color: var(--color-accent)" />
                <template v-if="isTestingKey || isConnecting">{{ t('onboarding.testingConnection') }}</template>
                <template v-else-if="isFetching">{{ t('onboarding.fetchingModels') }}</template>
                <template v-else-if="testKeyStatus === 'fail'">{{ t('onboarding.connectionFailed') }}{{ testKeyError ? ` (${testKeyError})` : '' }}</template>
                <template v-else>{{ t('onboarding.connectionFailed') }}{{ fetchError ? ` (${fetchError})` : '' }}</template>
              </p>
              <p v-if="isLocalProvider(providerForm, providerPresets)" class="mt-1.5" style="font-size: 10.5px; color: var(--color-text-muted); line-height: 1.4">
                {{ t('settings.localApiKeyHint') }}
              </p>
            </div>
            <div v-if="!isLocalProvider(providerForm, providerPresets)" class="api-disclaimer">
              <Info :size="11" :stroke-width="1.8" />
              <span>{{ t('settings.apiKeyDisclaimer') }}</span>
            </div>
          </div>

          <!-- Step 3: Lightweight model suggestion -->
          <div v-else-if="currentStep === 3" key="step3" class="flex flex-col items-center justify-center h-full py-10">
            <div class="w-12 h-12 rounded-full flex items-center justify-center mb-6" style="background: var(--color-accent-bg)">
              <PiggyBank :size="22" style="color: var(--color-accent)" />
            </div>
            <h2 class="text-xl font-medium mb-3" style="color: var(--color-text)">
              {{ t('onboarding.lightweightTitle') }}
            </h2>
            <p class="text-sm leading-relaxed text-center max-w-sm mb-4" style="color: var(--color-text-secondary)">
              {{ t('onboarding.lightweightBody') }}
            </p>
            <p class="text-xs text-center max-w-xs" style="color: var(--color-text-muted); line-height: 1.6">
              {{ t('onboarding.lightweightHint') }}
            </p>
          </div>

          <!-- Step 4: Select Models -->
          <div v-else-if="currentStep === 4" key="step4" class="flex flex-col py-6">
            <h2 class="text-lg font-medium mb-1" style="color: var(--color-text)">
              {{ t('onboarding.selectModelsTitle') }}
            </h2>
            <p class="text-sm mb-5" style="color: var(--color-text-secondary)">
              {{ t('onboarding.selectModelsBody') }}
            </p>

            <!-- Bulk actions -->
            <div class="flex gap-3 mb-4 items-center">
              <button
                @click="selectAll"
                class="text-xs font-medium px-3 py-1 rounded-md transition-colors"
                style="color: var(--color-accent); background: var(--color-accent-bg)"
              >
                {{ t('onboarding.selectAll') }}
              </button>
              <button
                @click="deselectAll"
                class="text-xs font-medium px-3 py-1 rounded-md transition-colors"
                style="color: var(--color-text-muted); background: var(--color-surface)"
              >
                {{ t('onboarding.deselectAll') }}
              </button>
            </div>

            <!-- Model list -->
            <div class="flex flex-col gap-1 max-h-56 overflow-y-auto pr-1">
              <label
                v-for="entry in availableModels"
                :key="entry.id"
                @click="toggleModel(entry)"
                class="flex items-center gap-3 h-9 px-3 rounded-lg cursor-pointer transition-colors text-sm"
                :style="{
                  background: selectedModels.has(entry.id) ? 'var(--color-accent-bg)' : 'transparent',
                  color: 'var(--color-text)',
                }"
              >
                <span
                  class="w-4 h-4 rounded flex items-center justify-center flex-shrink-0 transition-all"
                  :style="{
                    border: selectedModels.has(entry.id) ? '1.5px solid var(--color-accent)' : '1.5px solid var(--color-border)',
                    background: selectedModels.has(entry.id) ? 'var(--color-accent)' : 'transparent',
                  }"
                >
                  <Check v-if="selectedModels.has(entry.id)" :size="10" :stroke-width="3" style="color: white" />
                </span>
                <span class="truncate flex-1 min-w-0">{{ entry.id }}</span>
                <ModelCapabilityIcon :capabilities="entry.input_capabilities" />
              </label>

              <!-- Manual model input row (appended to list) -->
              <div class="manual-model-row">
                <span class="manual-model-check">
                  <Plus :size="12" :stroke-width="2.2" style="color: var(--color-accent)" />
                </span>
                <input
                  v-model="manualModelInput"
                  type="text"
                  class="manual-model-input"
                  :placeholder="t('onboarding.manualModelPlaceholder')"
                  @keydown.enter="addManualModel"
                />
                <button
                  @click="addManualModel"
                  :disabled="!manualModelInput.trim()"
                  class="manual-model-add"
                  :title="t('common.add')"
                >
                  <Plus :size="13" :stroke-width="2.2" />
                </button>
              </div>
            </div>

            <!-- Error + Retry -->
            <div v-if="fetchError" class="flex items-center gap-2 mt-3">
              <p class="text-xs" style="color: var(--color-danger)">
                {{ fetchError }}
              </p>
              <button
                @click="retryFetchModels"
                :disabled="isFetching"
                class="text-xs font-medium px-3 py-1 rounded-md transition-colors"
                style="color: var(--color-accent); background: var(--color-accent-bg)"
              >
                {{ t('onboarding.retryFetch') }}
              </button>
            </div>
          </div>

          <!-- Step 5: Search info -->
          <div v-else-if="currentStep === 5" key="step5" class="flex flex-col items-center justify-center h-full py-10">
            <div class="w-12 h-12 rounded-full flex items-center justify-center mb-6" style="background: var(--color-accent-bg)">
              <Globe :size="22" style="color: var(--color-accent)" />
            </div>
            <h2 class="text-xl font-medium mb-3" style="color: var(--color-text)">
              {{ t('onboarding.searchInfoTitle') }}
            </h2>
            <p class="text-sm leading-relaxed text-center max-w-sm mb-4" style="color: var(--color-text-secondary)">
              {{ t('onboarding.searchInfoBody') }}
            </p>
            <p class="text-xs text-center max-w-xs" style="color: var(--color-text-muted); line-height: 1.6">
              {{ t('onboarding.searchInfoHint') }}
            </p>
          </div>

          <!-- Step 6: Search API config -->
          <div v-else-if="currentStep === 6" key="step6" class="flex flex-col py-6">
            <h2 class="text-lg font-medium mb-6" style="color: var(--color-text)">
              {{ t('onboarding.addSearchTitle') }}
            </h2>

            <!-- Preset selector -->
            <div class="mb-5">
              <label class="block text-xs font-medium mb-1.5 tracking-wide uppercase" style="color: var(--color-text-muted)">
                {{ t('onboarding.searchPreset') }}
              </label>
              <div class="sel-wrap" style="position: relative">
                <button class="sel-btn w-full" @click="searchShowPresetMenu = !searchShowPresetMenu">
                  <span class="flex items-center gap-2 min-w-0 flex-1">
                    <component v-if="currentSearchPresetObj?.icon" :is="currentSearchPresetObj.icon" :size="14" :stroke-width="1.8" />
                    <span class="sel-text" :style="{ opacity: searchSelectedPreset ? 1 : 0.5 }">{{ currentSearchPresetLabel }}</span>
                  </span>
                  <ChevronDown :size="11" :stroke-width="2" class="sel-arrow" :class="{ rot: searchShowPresetMenu }" />
                </button>
                <Transition name="drop">
                  <div v-if="searchShowPresetMenu" class="sel-menu" style="position: absolute; top: 100%; left: 0; right: 0; margin-top: 4px; z-index: 10">
                    <div class="sel-clip">
                      <button
                        v-for="p in WEB_SEARCH_PRESETS" :key="p.id"
                        class="sel-opt"
                        :class="{ hit: searchSelectedPreset === p.id }"
                        @click="applySearchPreset(p.id)"
                      >
                        <div class="opt-left">
                          <component :is="p.icon" :size="14" :stroke-width="1.8" />
                          <div class="opt-info">
                            <span class="opt-id">{{ p.label }}</span>
                          </div>
                        </div>
                        <Check v-if="searchSelectedPreset === p.id" :size="13" :stroke-width="2.5" />
                      </button>
                    </div>
                  </div>
                </Transition>
              </div>
            </div>

            <!-- Name -->
            <div class="mb-4">
              <label class="block text-xs font-medium mb-1.5 tracking-wide uppercase" style="color: var(--color-text-muted)">
                {{ t('onboarding.searchName') }}
              </label>
              <input
                v-model="searchCustomName"
                type="text"
                class="w-full h-9 px-3 rounded-lg text-sm outline-none transition-colors select-text"
                style="background: var(--color-surface); color: var(--color-text); border: 1px solid var(--color-border)"
                :placeholder="searchSelectedPreset ? currentSearchPresetLabel : t('onboarding.providerName')"
              />
            </div>

            <!-- API Key (only after a preset is chosen — future providers may use
                 headless browsers, MCP, etc. instead of an API key) -->
            <div v-if="searchSelectedPreset" class="mb-4">
              <label class="block text-xs font-medium mb-1.5 tracking-wide uppercase" style="color: var(--color-text-muted)">
                {{ t('onboarding.apiKey') }}
              </label>
              <div class="flex gap-2">
                <div class="relative flex-1">
                  <input
                    v-model="searchApiKey"
                    :type="searchShowApiKey ? 'text' : 'password'"
                    class="w-full h-9 pl-3 pr-9 rounded-lg text-sm outline-none transition-colors select-text"
                    style="background: var(--color-surface); color: var(--color-text); border: 1px solid var(--color-border)"
                  />
                  <button
                    @click="searchShowApiKey = !searchShowApiKey"
                    class="absolute right-2 top-1/2 -translate-y-1/2 p-0.5 rounded opacity-40 hover:opacity-80 transition-opacity"
                    style="color: var(--color-text)"
                    tabindex="-1"
                  >
                    <Eye v-if="searchShowApiKey" :size="16" />
                    <EyeOff v-else :size="16" />
                  </button>
                </div>
                <button
                  class="flex items-center justify-center h-9 w-9 rounded-lg transition-colors"
                  :style="{
                    background: searchTestStatus === 'ok' ? 'var(--color-accent-bg)' : searchTestStatus === 'fail' ? 'rgba(239,68,68,0.1)' : 'var(--color-surface)',
                    border: '1px solid var(--color-border)',
                    cursor: (!searchSelectedPreset || searchKeyMissing || searchIsTesting) ? 'not-allowed' : 'pointer',
                    opacity: (!searchSelectedPreset || searchKeyMissing) ? 0.4 : 1,
                  }"
                  :disabled="!searchSelectedPreset || searchKeyMissing || searchIsTesting"
                  @click="testSearchConnection"
                  :title="t('settings.testConnection')"
                >
                  <Loader2 v-if="searchIsTesting" :size="14" class="spin" style="color: var(--color-accent)" />
                  <Check v-else-if="searchTestStatus === 'ok'" :size="14" style="color: var(--color-accent)" />
                  <Link2 v-else :size="14" style="color: var(--color-text-muted)" />
                </button>
              </div>
              <p v-if="currentSearchPresetObj?.apiUrl && searchSelectedPreset" class="mt-1.5" style="font-size: 10.5px; line-height: 1.4">
                <a :href="currentSearchPresetObj.apiUrl" target="_blank" rel="noopener noreferrer" style="color: var(--color-accent); text-decoration: underline; text-underline-offset: 2px;">
                  {{ t('settings.getApiKeyAt', { name: currentSearchPresetObj.label }) }}
                </a>
              </p>
              <p v-if="currentSearchPresetObj?.keyHelpKey" class="mt-1.5" style="font-size: 10.5px; color: var(--color-text-muted); line-height: 1.5">
                {{ t(currentSearchPresetObj.keyHelpKey) }}
              </p>
              <p v-if="searchIsTesting || searchTestStatus === 'fail'" class="text-xs mt-1.5 flex items-center gap-1.5" :style="{ color: searchTestStatus === 'fail' ? 'var(--color-danger)' : 'var(--color-text-secondary)' }">
                <Loader2 v-if="searchIsTesting" :size="14" class="spin" style="color: var(--color-accent)" />
                <template v-if="searchIsTesting">{{ t('onboarding.testingConnection') }}</template>
                <template v-else-if="searchTestStatus === 'fail'">{{ t('onboarding.connectionFailed') }}{{ searchTestError ? ` (${searchTestError})` : '' }}</template>
              </p>
            </div>
            <div v-if="searchSelectedPreset" class="api-disclaimer">
              <Info :size="11" :stroke-width="1.8" />
              <span>{{ t('settings.apiKeyDisclaimer') }}</span>
            </div>
          </div>

          <!-- Step 7: Done -->
          <div v-else-if="currentStep === 7" key="step7" class="flex flex-col items-center justify-center h-full py-10">
            <div class="w-12 h-12 rounded-full flex items-center justify-center mb-6" style="background: var(--color-accent-bg)">
              <PartyPopper :size="22" style="color: var(--color-accent)" />
            </div>
            <h2 class="text-xl font-medium mb-2" style="color: var(--color-text)">
              {{ t('onboarding.doneTitle') }}
            </h2>
            <p class="text-sm mb-4 text-center max-w-xs" style="color: var(--color-text-secondary)">
              {{ t('onboarding.doneBody', { shortcut: shortcutKey }) }}
            </p>
            <p class="text-xs mt-4" style="color: var(--color-text-muted)">
              {{ t('onboarding.shortcutHint') }}
            </p>
          </div>

          <!-- Step 8: Import (branch) -->
          <div v-else-if="currentStep === 8" key="step8" class="flex flex-col py-6">
            <!-- Success state: import done, stay here until Next (→ summary) -->
            <template v-if="importSucceeded">
              <div class="flex flex-col items-center justify-center h-full py-10 text-center">
                <div class="w-12 h-12 rounded-full flex items-center justify-center mb-6" style="background: var(--color-accent-bg)">
                  <Check :size="22" style="color: var(--color-accent)" />
                </div>
                <h2 class="text-xl font-medium mb-2" style="color: var(--color-text)">
                  {{ t('onboarding.importSuccessTitle') }}
                </h2>
                <p class="text-sm leading-relaxed max-w-xs" style="color: var(--color-text-secondary)">
                  {{ t('onboarding.importSuccessBody') }}
                </p>
                <button
                  @click="resetImportBranch"
                  class="flex items-center gap-1.5 h-9 px-4 rounded-lg text-sm font-medium transition-colors mt-6"
                  style="background: var(--color-surface); border: 1px solid var(--color-border); color: var(--color-text-secondary)"
                >
                  {{ t('onboarding.importRetry') }}
                </button>
              </div>
            </template>

            <!-- Import form (not yet succeeded) -->
            <template v-else>
              <h2 class="text-lg font-medium mb-2" style="color: var(--color-text)">
                {{ t('onboarding.importCardTitle') }}
              </h2>
              <p class="text-sm mb-1 leading-relaxed" style="color: var(--color-text-secondary)">
                {{ t('onboarding.importCardBody') }}
              </p>
              <p class="text-xs mb-4" style="color: var(--color-danger)">
                {{ t('settings.importData.import.warning') }}
              </p>

              <button
                v-if="!importPath"
                class="import-file-pick"
                @click="selectImportFile"
              >
                <FolderOpen :size="14" :stroke-width="1.8" />{{ t('settings.importData.import.selectFile') }}
              </button>

              <template v-else>
                <div class="import-file-row">
                  <FileText :size="14" class="import-file-icon" />
                  <span class="import-file-name" :title="importPath || ''">{{ importFileName }}</span>
                  <button
                    class="import-change-btn"
                    :disabled="importConfirming || importBusy || importAnalyzing"
                    @click="selectImportFile"
                    type="button"
                  >
                    <FolderOpen :size="13" />
                  </button>
                </div>

                <template v-if="!importConfirming">
                  <div class="import-pw-row mt-3">
                    <input
                      :type="importShowPw ? 'text' : 'password'"
                      class="import-pw-input"
                      v-model="importPassword"
                      :placeholder="t('settings.importData.import.passwordPlaceholder')"
                      autocomplete="off"
                      @keyup.enter="analyzeImport"
                    />
                    <button class="import-pw-toggle" @click="importShowPw = !importShowPw" type="button">
                      <Eye v-if="!importShowPw" :size="13" />
                      <EyeOff v-else :size="13" />
                    </button>
                  </div>

                  <button
                    class="import-btn mt-3"
                    :disabled="!importCanAnalyze"
                    @click="analyzeImport"
                  >
                    <Loader2 v-if="importAnalyzing" :size="12" class="spin" />
                    <Upload v-else :size="12" :stroke-width="1.9" />
                    {{ importAnalyzing
                      ? t('settings.importData.import.analyzing')
                      : importAnalyzed
                        ? t('settings.importData.import.reanalyze')
                        : t('settings.importData.import.analyze') }}
                  </button>

                  <template v-if="importAnalyzed">
                    <p class="text-xs mt-4 mb-2 font-medium" style="color: var(--color-text-secondary)">
                      {{ t('settings.importData.selectCategories') }}
                    </p>
                    <DataCategorySelector
                      v-model="importSelectedArray"
                      :available="importAvailableCats"
                      :counts="importPreview"
                    />

                    <button
                      class="import-btn danger mt-3"
                      :disabled="!importCanConfirm"
                      @click="requestImport"
                    >
                      <Loader2 v-if="importBusy" :size="12" class="spin" />
                      <Upload v-else :size="12" :stroke-width="1.9" />{{ t('settings.importData.import.button') }}
                    </button>
                  </template>
                </template>

                <div v-else class="import-confirm-row mt-3">
                  <div class="import-confirm-text">
                    <ShieldAlert :size="14" :stroke-width="1.6" />
                    <span>{{ t('settings.importData.import.confirmWarning') }}</span>
                  </div>
                  <div class="import-confirm-actions">
                    <button class="mini-btn" :title="t('common.cancel')" :disabled="importBusy" @click="stopCountdown">
                      <X :size="12" :stroke-width="2.5" />
                    </button>
                    <div class="import-confirm-cd">
                      <button
                        class="mini-btn danger-active"
                        :class="{ 'confirm-counting': importCountdown > 0 }"
                        :title="importCountdown > 0 ? t('settings.reset.confirmCountdown', { n: importCountdown }) : t('common.confirm')"
                        :disabled="importCountdown > 0 || importBusy"
                        @click="confirmImport"
                      >
                        <Loader2 v-if="importBusy" :size="12" class="spin" />
                        <Check v-else :size="12" :stroke-width="2.5" />
                      </button>
                      <span v-if="importCountdown > 0" class="import-cd-label">{{ importCountdown }}s</span>
                    </div>
                  </div>
                </div>
              </template>

              <p
                v-if="importStatus.kind !== 'idle'"
                class="mt-3 text-xs"
                :style="{ color: importStatus.kind === 'error' ? 'var(--color-danger)' : importStatus.kind === 'success' ? 'var(--color-accent)' : 'var(--color-text-muted)' }"
              >{{ importStatus.msg }}</p>
            </template>
          </div>

          <!-- Step 9: Import summary (branch) -->
          <div v-else-if="currentStep === 9" key="step9" class="flex flex-col py-6">
            <h2 class="text-lg font-medium mb-3" style="color: var(--color-text)">
              {{ t('onboarding.importSummaryTitle') }}
            </h2>
            <div class="flex flex-col gap-2 mb-4">
              <div class="flex items-center gap-2 text-sm" style="color: var(--color-text-secondary)">
                <Check v-if="hasUsableProviders" :size="14" style="color: var(--color-accent)" />
                <X v-else :size="14" style="color: var(--color-danger)" />
                <span>{{ t('onboarding.importSummaryProviders', { n: importedProviderCount }) }}</span>
              </div>
              <div class="flex items-center gap-2 text-sm" style="color: var(--color-text-secondary)">
                <Check v-if="hasWebSearch" :size="14" style="color: var(--color-accent)" />
                <X v-else :size="14" style="color: var(--color-danger)" />
                <span>{{ t('onboarding.importSummaryWebSearch', { n: importedWebSearchCount }) }}</span>
              </div>
            </div>
            <p class="text-sm leading-relaxed" style="color: var(--color-text-secondary)">
              <template v-if="hasUsableProviders && hasWebSearch">
                {{ t('onboarding.importSummaryAllReady') }}
              </template>
              <template v-else-if="hasUsableProviders && !hasWebSearch">
                {{ t('onboarding.importSummaryNeedWebSearch') }}
              </template>
              <template v-else>
                {{ t('onboarding.importSummaryNeedProvider') }}
              </template>
            </p>
          </div>

        </Transition>
        </div>
      </div>

      <!-- Bottom navigation -->
      <div class="flex items-center justify-between py-6">
        <!-- Previous button -->
        <button
          v-if="currentStep > 0"
          @click="goPrev"
          class="flex items-center gap-1.5 h-9 px-4 rounded-lg text-sm font-medium transition-colors"
          style="background: var(--color-surface); border: 1px solid var(--color-border); color: var(--color-text-secondary)"
        >
          <ChevronLeft :size="16" />
          {{ t('onboarding.previous') }}
        </button>
        <div v-else />

        <!-- Step dots (sequence depends on branch: 4 for import, 8 for add) -->
        <div class="flex items-center gap-2">
          <span
            v-for="(stepId, idx) in dotSteps"
            :key="stepId"
            class="w-1.5 h-1.5 rounded-full transition-all duration-300"
            :style="{
              background: idx <= currentDotIndex
                ? 'var(--color-accent)'
                : 'var(--color-border)',
              opacity: idx === currentDotIndex ? 1 : 0.5,
              transform: idx === currentDotIndex ? 'scale(1.4)' : 'scale(1)',
            }"
          />
        </div>

        <!-- Next button (step 6: Skip/Next in one button) -->
        <button
          v-if="currentStep === 6"
          @click="goNext"
          class="flex items-center gap-1.5 h-9 px-5 rounded-lg text-sm font-medium transition-all"
          :style="{
            background: searchCanNext ? 'var(--color-accent)' : 'var(--color-surface)',
            border: searchCanNext ? 'none' : '1px solid var(--color-border)',
            color: searchCanNext ? 'white' : 'var(--color-text-secondary)',
          }"
        >
          {{ searchCanNext ? t('onboarding.next') : t('onboarding.skip') }}
          <ChevronRight v-if="searchCanNext" :size="16" />
        </button>
        <button
          v-else-if="currentStep !== 8 || importSucceeded"
          @click="goNext"
          :disabled="!canProceed || isConnecting || isFetching"
          class="flex items-center gap-1.5 h-9 px-5 rounded-lg text-sm font-medium transition-all"
          :style="{
            background: (!canProceed || isConnecting || isFetching) ? 'var(--color-surface)' : 'var(--color-accent)',
            color: (!canProceed || isConnecting || isFetching) ? 'var(--color-text-muted)' : 'white',
            cursor: (!canProceed || isConnecting || isFetching) ? 'not-allowed' : 'pointer',
          }"
        >
          <Loader2 v-if="isConnecting || isFetching" :size="14" class="spin" />
          <template v-else>
            {{ isLastStep ? t('onboarding.finish') : t('onboarding.next') }}
            <ChevronRight v-if="!isLastStep" :size="16" />
          </template>
        </button>
        <!-- Step 8 (import) before success: no Next button — the in-card
             5s-countdown confirm button drives the actual import. -->
        <div v-else />
      </div>

    </div>
  </div>
</template>

<style scoped>
.mini-btn {
  display: flex; align-items: center; justify-content: center;
  width: 27px; height: 27px; border-radius: 7px;
  color: var(--color-text-muted); cursor: pointer;
  border: none; background: none; transition: .12s;
}
.mini-btn:hover { color: var(--color-text); background: var(--color-border); }
.mini-btn.danger-active {
  color: var(--color-danger); background: var(--color-danger-bg);
  animation: danger-pulse .8s ease-in-out infinite alternate;
}
@keyframes danger-pulse {
  to { background: var(--color-danger-bg); filter: brightness(.88); }
}

.slide-left-enter-active,
.slide-left-leave-active,
.slide-right-enter-active,
.slide-right-leave-active {
  transition: transform 0.3s ease, opacity 0.3s ease;
}

.slide-left-enter-from {
  transform: translateX(30px);
  opacity: 0;
}

.slide-left-leave-to {
  transform: translateX(-30px);
  opacity: 0;
}

.slide-right-enter-from {
  transform: translateX(-30px);
  opacity: 0;
}

.slide-right-leave-to {
  transform: translateX(30px);
  opacity: 0;
}

.spin {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

input {
  transition: border-color 0.15s ease;
}

input:focus {
  border-color: var(--color-accent) !important;
}

/* ── Manual model entry ── */
.manual-model-row {
  display: flex; align-items: center; gap: 12px;
  height: 36px; padding: 0 12px; border-radius: 10px;
  background: var(--color-surface);
  border: 1px dashed var(--color-border);
}
.manual-model-check {
  width: 16px; height: 16px; flex-shrink: 0;
  display: flex; align-items: center; justify-content: center;
}
.manual-model-input {
  flex: 1; min-width: 0; height: 28px; padding: 0 4px;
  background: transparent; color: var(--color-text);
  border: none; outline: none; font-size: 13px;
}
.manual-model-input::placeholder { color: var(--color-text-muted); }
.manual-model-add {
  display: flex; align-items: center; justify-content: center;
  width: 26px; height: 26px; border-radius: 7px; flex-shrink: 0;
  color: var(--color-accent); background: var(--color-accent-bg);
  border: none; cursor: pointer; transition: .12s;
}
.manual-model-add:hover:not(:disabled) { background: var(--color-accent-border); }
.manual-model-add:disabled { opacity: 0.4; cursor: default; }

/* Custom scrollbar for model list */
div::-webkit-scrollbar {
  width: 4px;
}

div::-webkit-scrollbar-track {
  background: transparent;
}

div::-webkit-scrollbar-thumb {
  background: var(--color-scrollbar);
  border-radius: 2px;
}

/* Scrollable step content: matches the 3px scrollbar used by the Settings /
 * data-management pages (ExportData / ImportData / ResetSoftware) so onboarding
 * does not look heavier than the rest of the app. The global 4px rule above
 * still applies to nested lists (e.g. model list). */
.onb-content::-webkit-scrollbar {
  width: 3px;
}
.onb-content::-webkit-scrollbar-track {
  background: transparent;
}
.onb-content::-webkit-scrollbar-thumb {
  background: var(--color-scrollbar);
  border-radius: 3px;
}
/* The scrollbar must hug the right edge of the card; any right padding would
 * push it inward. */
.onb-content {
  width: calc(100% + max(24px, 50vw - 260px));
  padding-right: 0 !important;
}

/* ── Dropdown (matching Settings style) ── */
.sel-btn {
  display:flex; align-items:center; gap:8px; width:100%;
  padding: 9px 13px; border-radius:9px; font-size:12px;
  background: var(--color-surface); border: 1px solid var(--color-scrollbar);
  color: var(--color-text); cursor:pointer; transition:.15s; text-align:left;
}
.sel-btn:hover{ border-color: var(--color-border-hover); background: var(--color-surface); }
.sel-text {
  flex:1; font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
  font-size: 11.5px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;
}
.sel-arrow { color: var(--color-text-muted); transition: transform .18s; flex-shrink:0; }
.sel-arrow.rot{ transform: rotate(180deg); }
.sel-menu {
  min-width:200px; max-width:320px; max-height:180px;
  padding: 0; border-radius: 11px;
  background: var(--color-overlay); backdrop-filter: blur(20px) saturate(1.4);
  border: 1px solid var(--color-border);
  box-shadow: 0 16px 40px rgba(0,0,0,.55), 0 0 0 1px var(--color-surface);
  overflow:hidden;
}
.sel-clip{ max-height:inherit; overflow-y:auto; overflow-x:hidden; padding:5px 7px 5px 5px; }
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

/* Variant selector (e.g. Region / Plan) for multi-variant family presets */
.ob-variant-row{ display:flex; align-items:center; gap:8px; margin-bottom:6px; }
.ob-variant-row:last-child{ margin-bottom:0; }
.ob-variant-label{
  font-size: 9.5px; font-weight: 600; text-transform: uppercase;
  letter-spacing: .055em; color: var(--color-text-muted); min-width: 52px;
}
.ob-variant-btns{ display:flex; gap:4px; }
.ob-variant-btn{
  font-size: 11px; padding: 3px 10px; border-radius: 6px;
  border: 1px solid var(--color-border); background: var(--color-surface);
  color: var(--color-text-muted); cursor: pointer;
  transition: border-color .15s, color .15s, background .15s;
}
.ob-variant-btn:hover{ color: var(--color-text); border-color: var(--color-accent-border); }
.ob-variant-btn.active{
  color: var(--color-accent); border-color: var(--color-accent-border);
  background: var(--color-accent-bg);
}
.opt-left{ display:flex; align-items:center; gap:8px; min-width:0; flex:1; }
.opt-info{ display:flex; flex-direction:column; gap:1px; min-width:0; }
.opt-id-row{ display:flex; align-items:center; gap:5px; min-width:0; }
.opt-id{
  font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
  font-size: 11.5px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;
}
.opt-series-tag{
  flex-shrink:0;
  font-size: 9px; font-weight: 600; letter-spacing: 0.02em;
  color: var(--color-text-muted);
  background: var(--color-surface-hover);
  padding: 0 5px; border-radius: 4px; line-height: 16px;
  white-space: nowrap;
}
.opt-src{ font-size: 9px; color: var(--color-text-muted); letter-spacing: .02em; }
.drop-enter-active,.drop-leave-active{ transition:opacity .14s ease,transform .14s ease; }
.drop-enter-from,.drop-leave-to{ opacity:0; transform: translateY(-5px) scale(.967); }

/* ── Theme toggle (mirrors Settings.vue) ── */
.theme-toggle {
  display: flex;
  gap: 1px;
  background: var(--color-border);
  border-radius: 9px;
  padding: 1px;
}
.theme-toggle.compact {
  flex: 0 1 auto;
  margin-bottom: 0;
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
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
}

.api-disclaimer {
  display: flex; align-items: flex-start; gap: 5px;
  margin-top: 8px;
  font-size: 10px; line-height: 1.45; color: var(--color-text-muted);
}
.api-disclaimer svg { flex-shrink: 0; margin-top: 1px; opacity: .65; }

/* ── Import step (branch) — reuses the onboarding visual language ── */
.import-file-pick {
  width: 100%;
  display: flex; align-items: center; justify-content: center; gap: 8px;
  height: 40px; border-radius: 10px;
  border: 1px dashed var(--color-border);
  background: var(--color-surface);
  color: var(--color-text-secondary);
  font-size: 13px; cursor: pointer; transition: .15s;
}
.import-file-pick:hover { border-color: var(--color-accent); color: var(--color-accent); }
.import-file-row {
  display: flex; align-items: center; gap: 8px;
  height: 36px; padding: 0 10px; border-radius: 10px;
  background: var(--color-surface); border: 1px solid var(--color-border);
}
.import-file-icon { color: var(--color-text-muted); flex-shrink: 0; }
.import-file-name {
  flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  color: var(--color-text); font-size: 12.5px;
}
.import-change-btn {
  display: flex; align-items: center; justify-content: center;
  width: 28px; height: 28px; border-radius: 7px; flex-shrink: 0;
  background: transparent; border: none;
  color: var(--color-text-muted); cursor: pointer; transition: .12s;
}
.import-change-btn:hover:not(:disabled) { color: var(--color-text); background: var(--color-border); }
.import-change-btn:disabled { opacity: 0.4; cursor: default; }
.import-pw-row { display: flex; align-items: center; gap: 8px; height: 36px; }
.import-pw-input {
  flex: 1; height: 36px; padding: 0 10px; border-radius: 10px;
  background: var(--color-surface); border: 1px solid var(--color-border);
  color: var(--color-text); font-size: 13px; outline: none;
  transition: border-color 0.15s ease;
}
.import-pw-input:focus { border-color: var(--color-accent); }
.import-pw-toggle {
  display: flex; align-items: center; justify-content: center;
  width: 36px; height: 36px; border-radius: 10px;
  background: var(--color-surface); border: 1px solid var(--color-border);
  color: var(--color-text-muted); cursor: pointer; transition: .12s;
}
.import-pw-toggle:hover { color: var(--color-text); }
.import-btn {
  display: flex; align-items: center; justify-content: center; gap: 6px;
  height: 36px; border-radius: 10px; border: none;
  font-size: 13px; font-weight: 500; cursor: pointer; transition: .12s;
  color: white; background: var(--color-danger);
}
.import-btn:hover:not(:disabled) { filter: brightness(1.05); }
.import-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.import-confirm-row {
  display: flex; flex-direction: column; gap: 10px;
  padding: 12px; border-radius: 10px;
  background: var(--color-danger-bg); border: 1px solid var(--color-danger);
}
.import-confirm-text {
  display: flex; align-items: flex-start; gap: 8px;
  color: var(--color-danger); font-size: 12px; line-height: 1.4;
}
.import-confirm-actions {
  display: flex; align-items: center; justify-content: flex-end; gap: 8px;
}
.import-confirm-cd { display: flex; align-items: center; gap: 6px; }
.import-cd-label { font-size: 11px; color: var(--color-danger); }
.mini-btn.confirm-counting { opacity: 0.6; cursor: not-allowed; }
</style>
