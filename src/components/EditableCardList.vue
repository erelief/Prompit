<script setup lang="ts">
import { ref, shallowRef, computed, watch, nextTick, triggerRef, type Component } from "vue";
import { useI18n } from "vue-i18n";
import draggable from "vuedraggable";
import {
  Plus, Trash2, Check, X, Pencil, GripVertical,
} from "@lucide/vue";

const { t } = useI18n();

const props = withDefaults(defineProps<{
  items: any[];
  title: string;
  icon: Component;
  emptyMessage?: string;
  emptySubMessage?: string;
  emptyIcon?: Component;
  /** Return an error message if the item is invalid, or null to allow. */
  validate?: (item: any) => string | null;
  /** Whether to show the remove button. Default true. */
  allowRemove?: boolean;
}>(), {
  allowRemove: true,
});

const emit = defineEmits<{
  add: [draft: any];
  confirm: [payload: { index: number }];
  cancel: [];
  remove: [payload: { index: number; indexMap: Map<number, number> }];
  "drag-end": [payload: { indexMap: Map<number, number> }];
}>();

defineOptions({ inheritAttrs: false });

// ── Internal state ──
const adding = ref(false);
const addDraft = ref<any>(null);
const editing = ref<Set<number>>(new Set());
const isEditingAny = computed(() => editing.value.size > 0);
const order = shallowRef<number[]>([]);
const pendingRemove = ref<number | null>(null);
const validationError = ref<string | null>(null);
const drafts = ref<Map<number, any>>(new Map());

watch(() => props.items.length, (len) => {
  order.value = Array.from({ length: len }, (_, i) => i);
}, { immediate: true });

// ── Editing ──
function isEditing(index: number): boolean {
  return editing.value.has(index);
}

function toggleEdit(index: number) {
  if (editing.value.has(index)) return;
  drafts.value.set(index, JSON.parse(JSON.stringify(props.items[index])));
  validationError.value = null;
  const s = new Set(editing.value);
  s.add(index);
  editing.value = s;
}

function confirmEdit(index: number) {
  const draft = drafts.value.get(index);
  if (!draft) return;
  if (props.validate) {
    const error = props.validate(draft);
    if (error) { validationError.value = error; return; }
  }
  validationError.value = null;
  Object.assign(props.items[index], draft);
  drafts.value.delete(index);
  const s = new Set(editing.value);
  s.delete(index);
  editing.value = s;
  emit("confirm", { index });
}

function cancelEdit(index: number) {
  validationError.value = null;
  drafts.value.delete(index);
  const s = new Set(editing.value);
  s.delete(index);
  editing.value = s;
}

// ── Add / Confirm / Cancel ──
function handleAdd() {
  validationError.value = null;
  const draft = {};
  emit("add", draft);
  addDraft.value = draft;
  nextTick(() => { adding.value = true; });
}

function handleConfirm() {
  if (props.validate) {
    const error = props.validate(addDraft.value);
    if (error) { validationError.value = error; return; }
  }
  validationError.value = null;
  const newIndex = props.items.length;
  props.items.push(addDraft.value);
  order.value.push(newIndex);
  addDraft.value = null;
  adding.value = false;
  emit("confirm", { index: newIndex });
}

function handleCancel() {
  validationError.value = null;
  addDraft.value = null;
  adding.value = false;
  emit("cancel");
}

// ── Remove (two-step confirm) ──
function requestRemove(index: number) {
  pendingRemove.value = index;
}
function confirmRemove(index: number) {
  pendingRemove.value = null;
  handleRemove(index);
}
function cancelRemove() {
  pendingRemove.value = null;
}
function handleRemove(index: number) {
  props.items.splice(index, 1);
  drafts.value.delete(index);
  const re = new Set<number>();
  for (const i of editing.value) {
    if (i === index) continue;
    re.add(i > index ? i - 1 : i);
  }
  editing.value = re;
  order.value = order.value.filter(i => i !== index).map(i => i > index ? i - 1 : i);
  emit("remove", { index, indexMap: buildIndexMap(props.items.length + 1, index) });
}

// ── Drag end ──
const rootEl = ref<HTMLElement | null>(null);

function onDragEnd() {
  const newOrder = order.value;
  const indexMap = new Map<number, number>();
  newOrder.forEach((oldIdx, newIdx) => indexMap.set(oldIdx, newIdx));

  // FLIP: First — capture current positions of each card
  const firstRects = new Map<number, DOMRect>();
  if (rootEl.value) {
    rootEl.value.querySelectorAll<HTMLElement>(".ecl-card").forEach(el => {
      const oi = Number(el.dataset.flipId);
      if (!isNaN(oi)) firstRects.set(oi, el.getBoundingClientRect());
    });
  }

  // Reorder the data array to match the visual order
  const reordered = newOrder.map(i => props.items[i]);
  for (let i = 0; i < reordered.length; i++) props.items[i] = reordered[i];
  // Remap editing set
  const re = new Set<number>();
  for (const i of editing.value) { const m = indexMap.get(i); if (m !== undefined) re.add(m); }
  editing.value = re;
  // Reset order to identity so rendering matches the now-reordered data
  order.value = props.items.map((_, i) => i);
  triggerRef(order);
  emit("drag-end", { indexMap });

  // FLIP: Invert + Play — animate cards from old to new positions
  nextTick(() => {
    if (!rootEl.value) return;
    rootEl.value.querySelectorAll<HTMLElement>(".ecl-card").forEach(el => {
      const oi = Number(el.dataset.flipId);
      if (isNaN(oi)) return;
      const first = firstRects.get(oi);
      if (!first) return;
      const last = el.getBoundingClientRect();
      const dx = first.left - last.left;
      const dy = first.top - last.top;
      if (!dx && !dy) return;
      el.style.transform = `translate(${dx}px, ${dy}px)`;
      el.style.transition = "none";
      // Force reflow so the "invert" transform applies before we animate
      el.offsetHeight; // eslint-disable-line no-unused-expressions
      el.style.transition = "transform 200ms ease";
      el.style.transform = "";
      el.addEventListener("transitionend", () => {
        el.style.transition = "";
        el.style.transform = "";
      }, { once: true });
    });
  });
}

// ── Helpers ──
function buildIndexMap(oldLen: number, removedAt: number): Map<number, number> {
  const m = new Map<number, number>();
  for (let old = 0; old < oldLen; old++) {
    if (old === removedAt) continue;
    m.set(old, old > removedAt ? old - 1 : old);
  }
  return m;
}
</script>

<template>
  <!-- Section head -->
  <div class="section-head" v-bind="$attrs">
    <span class="section-title"><component :is="icon" :size="13" />{{ title }}</span>
    <button class="pill-btn add-pill" @click="handleAdd" :disabled="adding">
      <Plus :size="12" :stroke-width="2" />{{ t('common.add') }} {{ title }}
    </button>
  </div>

  <div ref="rootEl" class="ecl-stack" :class="{ compact: !adding && !isEditingAny }">
    <!-- Empty state -->
    <div v-if="items.length === 0 && !adding" class="empty-card">
      <component :is="emptyIcon || icon" :size="22" :stroke-width="1" />
      <span>{{ emptyMessage || 'No items yet.' }}<br><small>{{ emptySubMessage || 'Add one to get started.' }}</small></span>
    </div>

    <!-- Adding form -->
    <div v-if="adding" class="ecl-card open">
      <div class="ecl-expanded">
        <div class="ecl-name-row">
          <slot name="name-input" :item="addDraft" :index="-1" :is-adding="true">
            <input v-model="addDraft.name" placeholder="Name…" class="name-input" @click.stop />
          </slot>
        </div>
        <slot name="content" :item="addDraft" :index="-1" :is-adding="true" />
        <div v-if="validationError" class="validation-error">{{ validationError }}</div>
        <div class="ecl-actions">
          <button class="mini-btn gold-active" :title="t('common.confirm')" @click.stop="handleConfirm">
            <Check :size="11" :stroke-width="2.5" />
          </button>
          <button class="mini-btn" :title="t('common.cancel')" @click.stop="handleCancel">
            <X :size="11" :stroke-width="2.5" />
          </button>
        </div>
      </div>
    </div>

    <!-- Cards -->
    <draggable
      :list="order"
      :item-key="(oi: number) => oi"
      handle=".card-drag-handle"
      ghost-class="card-ghost"
      :force-fallback="true"
      fallback-class="hidden-drag-ghost"
      class="drag-wrapper"
      :animation="200"
      :swap-threshold="0.5"
      :disabled="isEditingAny"
      @end="onDragEnd"
    >
      <template #item="{ element: oi }">
        <div
          v-show="!adding && !isEditingAny || isEditing(oi)"
          class="ecl-card"
          :class="{ open: isEditing(oi) }"
          :data-flip-id="oi"
        >
          <span class="card-drag-handle" @click.stop>
            <GripVertical :size="13" :stroke-width="1.8" />
          </span>

          <!-- Collapsed -->
          <div v-if="!isEditing(oi)" class="ecl-collapsed" :class="{ 'remove-pending': pendingRemove === oi }">
            <div class="ecl-lhs">
              <template v-if="pendingRemove === oi">
                <span class="remove-warning-text">{{ t('common.cannotBeUndone') }}</span>
              </template>
              <template v-else>
                <slot name="collapsed" :item="items[oi]" :index="oi" />
              </template>
            </div>
            <div class="ecl-rhs" @click.stop>
              <template v-if="pendingRemove === oi">
                <button class="mini-btn danger-active" :title="t('common.confirmRemove')" @click="confirmRemove(oi)">
                  <Check :size="11" :stroke-width="2.5" />
                </button>
                <button class="mini-btn" :title="t('common.cancel')" @click="cancelRemove">
                  <X :size="11" :stroke-width="2.5" />
                </button>
              </template>
              <template v-else>
                <button class="mini-btn" :title="t('common.edit')" @click="toggleEdit(oi)">
                  <Pencil :size="11" :stroke-width="1.9" />
                </button>
                <button v-if="allowRemove !== false" class="mini-btn warn" :title="t('common.remove')" @click="requestRemove(oi)">
                  <Trash2 :size="11" :stroke-width="1.9" />
                </button>
              </template>
            </div>
          </div>

          <!-- Expanded -->
          <div v-else class="ecl-expanded">
            <div class="ecl-name-row">
              <slot name="name-input" :item="drafts.get(oi)" :index="oi" :is-adding="false">
                <input :value="drafts.get(oi)?.name" @input="drafts.get(oi) && (drafts.get(oi)!.name = ($event.target as HTMLInputElement).value)" placeholder="Name…" class="name-input" @click.stop />
              </slot>
            </div>
            <slot name="content" :item="drafts.get(oi)" :index="oi" :is-adding="false" />
            <div v-if="validationError" class="validation-error">{{ validationError }}</div>
            <div class="ecl-actions">
              <button class="mini-btn gold-active" :title="t('common.confirm')" @click.stop="confirmEdit(oi)">
                <Check :size="11" :stroke-width="2.5" />
              </button>
              <button class="mini-btn" :title="t('common.cancel')" @click.stop="cancelEdit(oi)">
                <X :size="11" :stroke-width="2.5" />
              </button>
            </div>
          </div>
        </div>
      </template>
    </draggable>
  </div>
</template>

<style scoped>
/* ── Stack container ── */
.ecl-stack { display:flex; flex-direction:column; gap:7px; }
.ecl-stack.compact {
  max-height: 168px; overflow-y: auto; padding-right: 2px;
}
.ecl-stack.compact :deep(.ecl-card) { flex-shrink: 0; }
.ecl-stack.compact::-webkit-scrollbar { width: 3px; }
.ecl-stack.compact::-webkit-scrollbar-track { margin: 4px 0; }
.ecl-stack.compact::-webkit-scrollbar-thumb { background: var(--color-scrollbar); border-radius: 3px; }

.drag-wrapper { display: contents; }

/* ── Card ── */
.ecl-card {
  position: relative;
  border-radius: 11px; overflow:hidden;
  border: 1px solid var(--color-surface-hover);
  background: linear-gradient(180deg, var(--color-surface) 0%, var(--color-surface) 100%);
  transition: border-color .18s, box-shadow .18s;
  user-select: none;
}
.ecl-card:hover { border-color: var(--color-border-hover); }
.ecl-card.open { padding: 15px 30px 14px 40px; }

/* ── Collapsed ── */
.ecl-collapsed {
  display:flex; align-items:center; justify-content:space-between;
  padding: 11px 14px 11px 40px; transition:background .12s;
}
.ecl-collapsed:hover { background: var(--color-surface); }
.ecl-collapsed.remove-pending { background: var(--color-danger-bg); }
.ecl-lhs { display:flex; align-items:center; gap:10px; min-width:0; flex:1; }
.ecl-rhs { display:flex; align-items:center; gap:2px; opacity:.6; transition:opacity .12s; }
.ecl-collapsed:hover .ecl-rhs { opacity:1; }

/* ── Expanded / Adding ── */
.ecl-expanded { /* container only, no padding — card.open provides it */ }
.ecl-name-row {
  display:flex; align-items:center; gap:7px; margin-bottom:10px;
}
.ecl-actions {
  display:flex; align-items:center; justify-content:flex-end; gap:6px; margin-top:10px;
}

/* ── Drag handle ── */
.card-drag-handle {
  position: absolute; top: 12px; left: 10px;
  display: inline-flex; align-items: center; justify-content: center;
  width: 26px; height: 26px; border-radius: 5px;
  cursor: grab; color: var(--color-text-muted);
  z-index: 2; opacity: 0; transition: opacity 0.12s, color 0.12s;
  pointer-events: auto; user-select: none;
}
.ecl-card:hover > .card-drag-handle { opacity: 1; }
.ecl-card.open > .card-drag-handle { display: none; }
.card-drag-handle:hover {
  color: var(--color-text-secondary); background: var(--color-surface-hover);
}
.card-drag-handle:active { cursor: grabbing; color: var(--color-accent); }

/* ── Ghost / chosen ── */
:deep(.sortable-chosen) { opacity: 0.35; }
:deep(.sortable-ghost.card-ghost) {
  opacity: 0.35; background: var(--color-accent-bg);
  border: 1px dashed var(--color-accent-border);
  border-radius: 11px; min-height: 44px;
}

/* ── Pill button (Add / Confirm / Cancel) ── */
.pill-btn {
  display:inline-flex; align-items:center; gap:4px;
  padding: 4px 11px; border-radius: 7px; font-size: 10.5px; font-weight: 550;
  cursor: pointer; border:none; background:none; transition:.15s;
}
.add-pill { color: var(--color-accent-text); }
.add-pill:hover { color: var(--color-accent); background: var(--color-accent-bg); }
.add-pill:disabled { opacity:.32; cursor:default; }
.micro { color: var(--color-text-muted); padding: 3px 8px; }
.micro:hover:not(:disabled){ color: var(--color-text-secondary); background: var(--color-surface-hover); }
.micro:disabled{ opacity:.32; cursor:default; }
.gold-micro { color: var(--color-accent-text); }
.gold-micro:hover { color: var(--color-accent); background: var(--color-accent-bg); }

/* ── Mini button (Edit / Remove / Collapse) ── */
.mini-btn {
  display:flex; align-items:center; justify-content:center;
  width:27px; height:27px; border-radius:7px;
  color: var(--color-text-muted); cursor:pointer;
  border:none; background:none; transition:.12s;
}
.mini-btn:hover { color: var(--color-text); background: var(--color-border); }
.mini-btn.warn:hover { color: var(--color-danger); background: var(--color-danger-bg); }
.mini-btn.danger-active {
  color: var(--color-danger); background: var(--color-danger-bg);
  animation: danger-pulse .8s ease-in-out infinite alternate;
}
.remove-warning-row { margin-bottom: 6px; }
.remove-warning-text {
  font-size: 10px; font-weight: 550; letter-spacing: .01em;
  color: var(--color-danger);
}
.validation-error {
  font-size: 10px; font-weight: 550; letter-spacing: .01em;
  color: var(--color-accent); margin-top: 6px;
}
.mini-btn.ghost { color: var(--color-text-muted); }
.mini-btn.ghost:hover { color: var(--color-text-secondary); background: var(--color-surface); }
.mini-btn.gold-active { color: var(--color-accent-text); }
.mini-btn.gold-active:hover { color: var(--color-accent); background: var(--color-accent-bg); }

/* ── Name input ── */
.name-input {
  flex:1; background:none; border:none;
  font-size:14px; font-weight:700; letter-spacing: -.02em;
  color: var(--color-text); outline:none;
  padding:3px 5px; border-radius:5px; transition:background .15s;
}
.name-input::placeholder{ color: var(--color-text-muted); }
.name-input:focus{ background: var(--color-surface); }

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

/* ── Empty state ── */
.empty-card {
  display:flex; flex-direction:column; align-items:center; gap:8px;
  padding: 28px 16px; border-radius: 11px;
  border: 1px dashed var(--color-surface-hover);
  color: var(--color-text-muted); font-size: 11.5px; line-height: 1.5;
  text-align: center;
}
.empty-card small{ font-size: 10px; color: var(--color-text-muted); }

@keyframes spin{ to{ transform: rotate(360deg)} }
@keyframes danger-pulse{ to{ background: var(--color-danger-bg)} }
.spin{ animation: spin .75s linear infinite; }
</style>
