<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import { Check, X } from "@lucide/vue";
import type { ProviderConfig, ProviderModel } from "../stores/config";

/**
 * Raw-JSON editor for a provider config (advanced users). Opened from the
 * provider add/edit forms in Settings and Onboarding; surfaces the fields the
 * GUI does not expose (temperature, max_tokens, api_format, …).
 *
 * The parent v-if's this component, so mount = open: the textarea is seeded
 * once from the current config. Apply emits a normalized ProviderConfig —
 * only known keys survive, and type-mismatched values throw so a typo never
 * silently rewrites the provider.
 */
const props = defineProps<{ config: ProviderConfig }>();
const emit = defineEmits<{
  apply: [config: ProviderConfig];
  close: [];
}>();

const { t } = useI18n();

const text = ref(JSON.stringify(props.config, null, 2));
const error = ref("");

function fail(msg: string): never {
  throw new Error(msg);
}

function optionalString(parsed: Record<string, unknown>, key: string): string | undefined {
  const v = parsed[key];
  if (v === undefined) return undefined;
  if (typeof v !== "string") fail(`"${key}" must be a string`);
  return v;
}

function optionalNumberOrNull(parsed: Record<string, unknown>, key: string): number | null | undefined {
  const v = parsed[key];
  if (v === undefined) return undefined;
  if (v !== null && typeof v !== "number") fail(`"${key}" must be a number or null`);
  return v;
}

/** Parse edited JSON into a clean ProviderConfig. Syntax errors propagate
 *  with their native message; shape errors throw with a key-specific one. */
function parseProviderConfigJson(raw: string): ProviderConfig {
  const parsed: unknown = JSON.parse(raw);
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    fail("expected a JSON object");
  }
  const p = parsed as Record<string, unknown>;

  const out: ProviderConfig = {
    name: optionalString(p, "name") ?? "",
    api_key: optionalString(p, "api_key") ?? "",
    base_url: optionalString(p, "base_url") ?? "",
    models: [],
    temperature: optionalNumberOrNull(p, "temperature") ?? null,
    max_tokens: optionalNumberOrNull(p, "max_tokens") ?? null,
  };

  if (p.models !== undefined) {
    if (!Array.isArray(p.models)) fail('"models" must be an array');
    out.models = p.models
      .filter((m): m is { id: string; input_capabilities?: unknown } =>
        !!m && typeof m === "object" && typeof (m as any).id === "string")
      .map((m): ProviderModel => ({
        id: m.id,
        ...(m.input_capabilities && typeof m.input_capabilities === "object"
          ? { input_capabilities: m.input_capabilities as ProviderModel["input_capabilities"] }
          : {}),
      }));
  }

  const preset = optionalString(p, "preset");
  if (preset !== undefined) out.preset = preset;
  if (p.api_format !== undefined) {
    if (!p.api_format || typeof p.api_format !== "object" || Array.isArray(p.api_format)) {
      fail('"api_format" must be an object');
    }
    out.api_format = p.api_format as ProviderConfig["api_format"];
  }
  return out;
}

function apply() {
  try {
    const cfg = parseProviderConfigJson(text.value);
    error.value = "";
    emit("apply", cfg);
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  }
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") {
    e.preventDefault();
    emit("close");
  }
}
onMounted(() => window.addEventListener("keydown", onKeydown));
onUnmounted(() => window.removeEventListener("keydown", onKeydown));
</script>

<template>
  <Teleport to="body">
    <Transition name="drop">
      <div class="modal-overlay" @click.self="emit('close')">
        <div class="modal-card json-modal">
          <div class="json-title">{{ t('settings.configJsonTitle') }}</div>
          <div class="json-hint">{{ t('settings.configJsonHint') }}</div>
          <textarea v-model="text" class="json-editor" spellcheck="false"></textarea>
          <div v-if="error" class="json-error">{{ t('settings.configJsonInvalid') }}: {{ error }}</div>
          <div class="json-actions">
            <button class="mini-btn" :title="t('common.cancel')" @click="emit('close')">
              <X :size="12" :stroke-width="2.5" />
            </button>
            <button class="mini-btn json-confirm" :title="t('common.confirm')" @click="apply">
              <Check :size="12" :stroke-width="2.5" />
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.json-modal {
  width: 480px;
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.json-title {
  font-size: var(--text-lg);
  font-weight: var(--weight-bold);
  letter-spacing: -0.02em;
  color: var(--color-text);
}

.json-hint {
  font-size: var(--text-sm);
  line-height: 1.5;
  color: var(--color-text-muted);
}

.json-editor {
  width: 100%;
  height: 260px;
  resize: vertical;
  box-sizing: border-box;
  padding: var(--space-2) var(--space-3);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-surface);
  color: var(--color-text);
  font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
  font-size: 11.5px;
  line-height: 1.6;
  outline: none;
  white-space: pre;
  overflow: auto;
  scrollbar-width: thin;
  scrollbar-color: var(--color-scrollbar) transparent;
}

.json-editor::-webkit-scrollbar { width: 3px; }
.json-editor::-webkit-scrollbar-thumb { background: var(--color-scrollbar); border-radius: var(--radius-xs); }

.json-editor:focus {
  border-color: var(--color-accent-border);
  box-shadow: 0 0 0 2px var(--color-accent-bg);
}

.json-error {
  font-size: var(--text-xs);
  font-weight: var(--weight-medium);
  color: var(--color-danger);
  word-break: break-all;
}

.json-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 6px;
}

.json-confirm {
  color: var(--color-accent-text);
}

.json-confirm:hover {
  color: var(--color-accent);
  background: var(--color-accent-bg);
}
</style>
