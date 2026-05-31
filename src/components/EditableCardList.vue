<script setup lang="ts">
import { ref, shallowRef, computed, watch, nextTick, triggerRef, type Component } from "vue";
import draggable from "vuedraggable";
import {
  Plus, Trash2, Check, ChevronDown, Pencil, GripVertical,
} from "@lucide/vue";

const props = defineProps<{
  items: any[];
  title: string;
  icon: Component;
  emptyMessage?: string;
  emptySubMessage?: string;
  emptyIcon?: Component;
  /** Return an error message if the item is invalid, or null to allow. */
  validate?: (item: any) => string | null;
}>();

const emit = defineEmits<{
  add: [];
  confirm: [payload: { index: number }];
  cancel: [payload: { index: number; indexMap: Map<number, number> }];
  remove: [payload: { index: number; indexMap: Map<number, number> }];
  "drag-end": [payload: { indexMap: Map<number, number> }];
}>();

defineOptions({ inheritAttrs: false });

// ── Internal state ──
const adding = ref(false);
const editing = ref<Set<number>>(new Set());
const isEditingAny = computed(() => editing.value.size > 0);
const order = shallowRef<number[]>([]);
const pendingRemove = ref<number | null>(null);
const validationError = ref<string | null>(null);

watch(() => props.items.length, (len) => {
  order.value = Array.from({ length: len }, (_, i) => i);
}, { immediate: true });

const NI = computed(() => props.items.length - 1);

// ── Editing ──
function isEditing(index: number): boolean {
  return editing.value.has(index);
}

function toggleEdit(index: number) {
  // Collapsing — validate first
  if (editing.value.has(index)) {
    const item = props.items[index];
    if (props.validate) {
      const error = props.validate(item);
      if (error) { validationError.value = error; return; }
    }
    validationError.value = null;
  } else {
    validationError.value = null;
  }
  const s = new Set(editing.value);
  s.has(index) ? s.delete(index) : s.add(index);
  editing.value = s;
}

// ── Add / Confirm / Cancel ──
function handleAdd() {
  validationError.value = null;
  emit("add");
  nextTick(() => { adding.value = true; });
}

function handleConfirm() {
  const item = props.items[NI.value];
  if (props.validate) {
    const error = props.validate(item);
    if (error) { validationError.value = error; return; }
  }
  validationError.value = null;
  adding.value = false;
  emit("confirm", { index: NI.value });
}

function handleCancel() {
  validationError.value = null;
  const idx = NI.value;
  props.items.pop();
  order.value.pop();
  editing.value.delete(idx);
  adding.value = false;
  const indexMap = buildIndexMap(props.items.length, -1);
  emit("cancel", { index: idx, indexMap });
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
function onDragEnd() {
  const newOrder = order.value;
  const indexMap = new Map<number, number>();
  newOrder.forEach((oldIdx, newIdx) => indexMap.set(oldIdx, newIdx));
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
      <Plus :size="12" :stroke-width="2" />Add {{ title }}
    </button>
  </div>

  <div class="ecl-stack" :class="{ compact: !adding && !isEditingAny }">
    <!-- Empty state -->
    <div v-if="items.length === 0 && !adding" class="empty-card">
      <component :is="emptyIcon || icon" :size="22" :stroke-width="1" />
      <span>{{ emptyMessage || 'No items yet.' }}<br><small>{{ emptySubMessage || 'Add one to get started.' }}</small></span>
    </div>

    <!-- Adding form -->
    <div v-if="adding" class="ecl-card open">
      <div class="ecl-expanded">
        <div class="ecl-name-row">
          <slot name="name-input" :item="items[NI]" :index="NI" :is-adding="true">
            <input v-model="items[NI].name" placeholder="Name…" class="name-input" @click.stop />
          </slot>
        </div>
        <slot name="content" :item="items[NI]" :index="NI" :is-adding="true" />
        <div v-if="validationError" class="validation-error">{{ validationError }}</div>
        <div class="ecl-actions">
          <button class="pill-btn gold-micro" @click.stop="handleConfirm">
            <Check :size="10" :stroke-width="2.5" />Confirm
          </button>
          <button class="pill-btn micro" @click.stop="handleCancel">Cancel</button>
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
      :swap-threshold="0.65"
      :disabled="isEditingAny"
      @end="onDragEnd"
    >
      <template #item="{ element: oi }">
        <div
          v-show="!adding && !isEditingAny || isEditing(oi)"
          class="ecl-card"
          :class="{ open: isEditing(oi) }"
        >
          <span class="card-drag-handle" @click.stop>
            <GripVertical :size="13" :stroke-width="1.8" />
          </span>

          <!-- Collapsed -->
          <div v-if="!isEditing(oi)" class="ecl-collapsed" :class="{ 'remove-pending': pendingRemove === oi }" @click="pendingRemove === oi || toggleEdit(oi)">
            <div class="ecl-lhs">
              <template v-if="pendingRemove === oi">
                <span class="remove-warning-text">This cannot be undone.</span>
              </template>
              <template v-else>
                <slot name="collapsed" :item="items[oi]" :index="oi" />
              </template>
            </div>
            <div class="ecl-rhs" @click.stop>
              <template v-if="pendingRemove === oi">
                <button class="mini-btn danger-active" title="Confirm remove" @click="confirmRemove(oi)">
                  <Check :size="11" :stroke-width="2.5" />
                </button>
                <button class="mini-btn" title="Cancel" @click="cancelRemove">
                  <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
                </button>
              </template>
              <template v-else>
                <button class="mini-btn" title="Edit" @click="toggleEdit(oi)">
                  <Pencil :size="11" :stroke-width="1.9" />
                </button>
                <button class="mini-btn warn" title="Remove" @click="requestRemove(oi)">
                  <Trash2 :size="11" :stroke-width="1.9" />
                </button>
              </template>
            </div>
          </div>

          <!-- Expanded -->
          <div v-else class="ecl-expanded">
            <div class="ecl-name-row">
              <slot name="name-input" :item="items[oi]" :index="oi" :is-adding="false">
                <input v-model="items[oi].name" placeholder="Name…" class="name-input" @click.stop />
              </slot>
              <button class="mini-btn ghost" title="Collapse" @click.stop="toggleEdit(oi)">
                <ChevronDown :size="14" :stroke-width="1.8" class="chev-up" />
              </button>
              <template v-if="pendingRemove === oi">
                <button class="mini-btn danger-active" title="Confirm remove" @click.stop="confirmRemove(oi)">
                  <Check :size="12" :stroke-width="2.5" />
                </button>
                <button class="mini-btn ghost" title="Cancel" @click.stop="cancelRemove">
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
                </button>
              </template>
              <template v-else>
                <button class="mini-btn warn" title="Remove" @click.stop="requestRemove(oi)">
                  <Trash2 :size="12" :stroke-width="1.8" />
                </button>
              </template>
            </div>
            <div v-if="pendingRemove === oi" class="remove-warning-row">
              <span class="remove-warning-text">This cannot be undone.</span>
            </div>
            <slot name="content" :item="items[oi]" :index="oi" :is-adding="false" />
            <div v-if="validationError" class="validation-error">{{ validationError }}</div>
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
.ecl-stack.compact::-webkit-scrollbar-thumb { background: rgba(255,255,255,.1); border-radius: 3px; }

.drag-wrapper { display: contents; }

/* ── Card ── */
.ecl-card {
  position: relative;
  border-radius: 11px; overflow:hidden;
  border: 1px solid rgba(255,255,255,.055);
  background: linear-gradient(180deg, rgba(255,255,255,.022) 0%, rgba(255,255,255,.014) 100%);
  transition: border-color .18s, box-shadow .18s;
  user-select: none;
}
.ecl-card:hover { border-color: rgba(255,255,255,.09); }
.ecl-card.open { padding: 15px 30px 14px 40px; }

/* ── Collapsed ── */
.ecl-collapsed {
  display:flex; align-items:center; justify-content:space-between;
  padding: 11px 14px 11px 40px; cursor:pointer; transition:background .12s;
}
.ecl-collapsed:hover { background: rgba(255,255,255,.02); }
.ecl-collapsed.remove-pending { background: rgba(248,113,113,.06); cursor: default; }
.ecl-lhs { display:flex; align-items:center; gap:10px; min-width:0; flex:1; }
.ecl-rhs { display:flex; align-items:center; gap:2px; opacity:.6; transition:opacity .12s; }
.ecl-collapsed:hover .ecl-rhs { opacity:1; }

/* ── Expanded / Adding ── */
.ecl-expanded { /* container only, no padding — card.open provides it */ }
.ecl-name-row {
  display:flex; align-items:center; gap:7px; margin-bottom:10px;
}
.ecl-actions {
  display:flex; align-items:center; gap:6px; margin-top:10px;
}

/* ── Drag handle ── */
.card-drag-handle {
  position: absolute; top: 12px; left: 10px;
  display: inline-flex; align-items: center; justify-content: center;
  width: 26px; height: 26px; border-radius: 5px;
  cursor: grab; color: rgba(255, 255, 255, 0.18);
  z-index: 2; opacity: 0; transition: opacity 0.12s, color 0.12s;
  pointer-events: auto; user-select: none;
}
.ecl-card:hover > .card-drag-handle { opacity: 1; }
.card-drag-handle:hover {
  color: rgba(255, 255, 255, 0.45); background: rgba(255, 255, 255, 0.06);
}
.card-drag-handle:active { cursor: grabbing; color: rgba(255, 255, 255, 0.55); }

/* ── Ghost / chosen ── */
:deep(.sortable-chosen) { opacity: 0.35; }
:deep(.sortable-ghost.card-ghost) {
  opacity: 0.35; background: rgba(212, 160, 72, 0.10);
  border: 1px dashed rgba(212, 160, 72, 0.35);
  border-radius: 11px; min-height: 44px;
}

/* ── Pill button (Add / Confirm / Cancel) ── */
.pill-btn {
  display:inline-flex; align-items:center; gap:4px;
  padding: 4px 11px; border-radius: 7px; font-size: 10.5px; font-weight: 550;
  cursor: pointer; border:none; background:none; transition:.15s;
}
.add-pill { color: rgba(212,160,72,.72); }
.add-pill:hover { color: #d4a048; background: rgba(212,160,72,.09); }
.add-pill:disabled { opacity:.32; cursor:default; }
.micro { color: rgba(255,255,255,.28); padding: 3px 8px; }
.micro:hover:not(:disabled){ color: rgba(255,255,255,.52); background: rgba(255,255,255,.055); }
.micro:disabled{ opacity:.32; cursor:default; }
.gold-micro { color: rgba(212,160,72,.62); }
.gold-micro:hover { color: rgba(212,160,72,.9); background: rgba(212,160,72,.08); }

/* ── Mini button (Edit / Remove / Collapse) ── */
.mini-btn {
  display:flex; align-items:center; justify-content:center;
  width:27px; height:27px; border-radius:7px;
  color: rgba(255,255,255,.32); cursor:pointer;
  border:none; background:none; transition:.12s;
}
.mini-btn:hover { color: rgba(255,255,255,.7); background: rgba(255,255,255,.065); }
.mini-btn.warn:hover { color: #f87171; background: rgba(248,113,113,.1); }
.mini-btn.danger-active {
  color: #f87171; background: rgba(248,113,113,.14);
  animation: danger-pulse .8s ease-in-out infinite alternate;
}
.remove-warning-row { margin-bottom: 6px; }
.remove-warning-text {
  font-size: 10px; font-weight: 550; letter-spacing: .01em;
  color: rgba(248,113,113,.65);
}
.validation-error {
  font-size: 10px; font-weight: 550; letter-spacing: .01em;
  color: rgba(212,160,72,.65); margin-top: 6px;
}
.mini-btn.ghost { color: rgba(255,255,255,.2); }
.mini-btn.ghost:hover { color: rgba(255,255,255,.48); background: rgba(255,255,255,.045); }

/* ── Name input ── */
.name-input {
  flex:1; background:none; border:none;
  font-size:14px; font-weight:700; letter-spacing: -.02em;
  color: rgba(255,255,255,.86); outline:none;
  padding:3px 5px; border-radius:5px; transition:background .15s;
}
.name-input::placeholder{ color: rgba(255,255,255,.2); }
.name-input:focus{ background: rgba(255,255,255,.045); }

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

/* ── Empty state ── */
.empty-card {
  display:flex; flex-direction:column; align-items:center; gap:8px;
  padding: 28px 16px; border-radius: 11px;
  border: 1px dashed rgba(255,255,255,.06);
  color: rgba(255,255,255,.18); font-size: 11.5px; line-height: 1.5;
  text-align: center;
}
.empty-card small{ font-size: 10px; color: rgba(255,255,255,.13); }

@keyframes spin{ to{ transform: rotate(360deg)} }
@keyframes danger-pulse{ to{ background: rgba(248,113,113,.24)} }
.spin{ animation: spin .75s linear infinite; }
.chev-up{ transform: rotate(180deg); }
</style>
