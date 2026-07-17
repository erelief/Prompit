<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { Check, Minus } from "@lucide/vue";
import {
  ALL_CATEGORIES,
  CATEGORY_META,
  type DataCategory,
  type CategoryPreview,
} from "../composables/useDataCategories";

const props = withDefaults(
  defineProps<{
    /** Selected category ids (v-model). */
    modelValue: string[];
    /** Categories available to choose from. Defaults to all. */
    available?: DataCategory[];
    /** Per-category counts to display (import mode, from BundlePreview). */
    counts?: CategoryPreview[];
    /** Disable all interaction. */
    disabled?: boolean;
  }>(),
  { available: () => [...ALL_CATEGORIES], disabled: false },
);

const emit = defineEmits<{ "update:modelValue": [string[]] }>();

const { t } = useI18n();

const countMap = computed<Record<string, number | null>>(() => {
  const m: Record<string, number | null> = {};
  for (const c of props.counts ?? []) m[c.id] = c.count;
  return m;
});

const selectedSet = computed(() => new Set(props.modelValue));

const allSelected = computed(
  () => props.available.length > 0 && props.available.every((c) => selectedSet.value.has(c)),
);
const someSelected = computed(
  () => !allSelected.value && props.available.some((c) => selectedSet.value.has(c)),
);

function toggle(cat: DataCategory) {
  if (props.disabled) return;
  const next = new Set(props.modelValue);
  if (next.has(cat)) next.delete(cat);
  else next.add(cat);
  emit("update:modelValue", [...next]);
}

function toggleAll() {
  if (props.disabled) return;
  if (allSelected.value) {
    // Deselect only the visible ones, keep any external selections.
    const hidden = new Set(props.modelValue);
    for (const c of props.available) hidden.delete(c);
    emit("update:modelValue", [...hidden]);
  } else {
    const next = new Set(props.modelValue);
    for (const c of props.available) next.add(c);
    emit("update:modelValue", [...next]);
  }
}

function countLabel(cat: DataCategory): string | null {
  const c = countMap.value[cat];
  if (c === undefined) return null; // category not in bundle
  if (c === null) return null; // opaque shape
  return String(c);
}
</script>

<template>
  <div class="cat-selector">
    <button
      type="button"
      class="cat-row cat-all"
      :class="{ checked: allSelected, indeterminate: someSelected, disabled }"
      :disabled="disabled"
      @click="toggleAll"
    >
      <span class="cat-check">
        <Check v-if="allSelected" :size="11" :stroke-width="3" />
        <Minus v-else-if="someSelected" :size="11" :stroke-width="3" />
      </span>
      <span class="cat-label">{{ t('settings.categories.selectAll') }}</span>
    </button>

    <button
      v-for="cat in available"
      :key="cat"
      type="button"
      class="cat-row"
      :class="{
        checked: selectedSet.has(cat),
        disabled,
        absent: counts && countMap[cat] === undefined,
      }"
      :disabled="disabled"
      @click="toggle(cat)"
    >
      <span class="cat-check">
        <Check v-if="selectedSet.has(cat)" :size="11" :stroke-width="3" />
      </span>
      <span class="cat-text">
        <span class="cat-label-line">
          <span class="cat-label">{{ t(CATEGORY_META[cat].labelKey) }}</span>
          <span v-if="countLabel(cat) !== null" class="cat-count">{{ countLabel(cat) }}</span>
          <span v-if="CATEGORY_META[cat].sensitive" class="cat-badge sensitive">{{ t('settings.categories.sensitive') }}</span>
          <span v-if="counts && countMap[cat] === undefined" class="cat-badge absent">{{ t('settings.categories.notInBackup') }}</span>
        </span>
        <span class="cat-desc">{{ t(CATEGORY_META[cat].descKey) }}</span>
      </span>
    </button>
  </div>
</template>

<style scoped>
.cat-selector {
  display: flex;
  flex-direction: column;
  gap: 4px;
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 7px;
}
.cat-row {
  display: flex;
  align-items: flex-start;
  gap: 9px;
  width: 100%;
  padding: 7px 8px;
  border-radius: var(--radius-sm);
  border: none;
  background: none;
  cursor: pointer;
  text-align: left;
  font-family: inherit;
  transition: background 0.12s;
}
.cat-row:hover:not(.disabled) {
  background: var(--color-surface-hover);
}
.cat-row.disabled {
  cursor: not-allowed;
}
.cat-row.disabled .cat-label,
.cat-row.disabled .cat-desc {
  opacity: 0.55;
}
.cat-row.checked {
  background: var(--color-surface);
}
.cat-all {
  border-bottom: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  padding-bottom: 8px;
  margin-bottom: 2px;
  align-items: center;
}
.cat-all .cat-label {
  font-weight: 650;
}

.cat-check {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  border-radius: var(--radius-xs);
  border: 1.5px solid var(--color-border);
  background: var(--color-bg);
  color: var(--color-bg);
  flex-shrink: 0;
  margin-top: 1px;
  transition: 0.12s;
}
.cat-row.checked .cat-check,
.cat-row.indeterminate .cat-check {
  border-color: var(--color-accent-border);
  background: var(--color-accent);
}

.cat-text {
  display: flex;
  flex-direction: column;
  gap: 1px;
  flex: 1;
  min-width: 0;
}
.cat-label-line {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}
.cat-label {
  font-size: 11.5px;
  font-weight: 600;
  color: var(--color-text);
}
.cat-count {
  font-size: 10px;
  font-weight: 650;
  color: var(--color-accent-text);
  background: var(--color-accent-bg);
  padding: 1px 6px;
  border-radius: var(--radius-md);
  font-variant-numeric: tabular-nums;
}
.cat-desc {
  font-size: 10.5px;
  font-weight: 500;
  color: var(--color-text-muted);
  line-height: 1.4;
}
.cat-badge {
  font-size: 9px;
  font-weight: 650;
  letter-spacing: 0.03em;
  padding: 1px 5px;
  border-radius: var(--radius-xs);
  text-transform: uppercase;
}
.cat-badge.sensitive {
  color: var(--color-danger);
  background: var(--color-danger-bg);
}
.cat-badge.absent {
  color: var(--color-text-muted);
  background: transparent;
  border: 1px dashed var(--color-border);
}
.cat-row.absent {
  opacity: 0.6;
}
</style>
