<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ArrowLeft, Info, CalendarDays, PieChart, Cpu, Check } from "@lucide/vue";
import { useSettingsWindow } from "../composables/useSettingsWindow";
import { usageStore, loadUsage, MODES } from "../stores/config";
import type { UsageRecord } from "../stores/config";
import { hostOf } from "../services/llm-client";

const { t, locale } = useI18n();
const router = useRouter();
const { growAbove } = useSettingsWindow(620);

onMounted(() => {
  // Force a fresh read so imports / resets that replaced the file are seen.
  loadUsage(true);
});

// ── View state ──
type ViewKey = "date" | "mode" | "model";
const view = ref<ViewKey>("date");
const days = ref<7 | 30>(7);
/** Provider keys excluded from the date view; empty = all providers included
 *  (default, i.e. the grand total across providers). */
const excludedProviders = ref<Set<string>>(new Set());

const DAY_MS = 24 * 60 * 60 * 1000;
function startOfDay(ts: number): number {
  const d = new Date(ts);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}

/** Stable provider identity of a record; pre-provider_key records fall back
 *  to the display name. */
function providerKeyOf(r: UsageRecord): string {
  return r.provider_key ?? r.provider;
}

/** All stats on this page cover the last 30 days (by calendar day). */
const records30 = computed<UsageRecord[]>(() => {
  const cutoff = startOfDay(Date.now()) - 29 * DAY_MS;
  return usageStore.records.filter((r) => r.ts >= cutoff);
});

/** Display label per provider key. Two providers sharing one display name are
 *  disambiguated with the endpoint host so the checklist stays readable. */
const providerLabels = computed<Map<string, string>>(() => {
  const names = new Map<string, string>();
  for (const r of records30.value) {
    const k = providerKeyOf(r);
    if (!names.has(k)) names.set(k, r.provider);
  }
  const counts = new Map<string, number>();
  for (const n of names.values()) counts.set(n, (counts.get(n) ?? 0) + 1);
  const out = new Map<string, string>();
  for (const [k, n] of names) {
    const host = k.includes("|") ? hostOf(k.split("|")[1]) : "";
    out.set(k, (counts.get(n) ?? 0) > 1 && host ? `${n} (${host})` : n);
  }
  return out;
});

/** Date view window: 7 or 30 days. */
const windowRecords = computed<UsageRecord[]>(() => {
  const cutoff = startOfDay(Date.now()) - (days.value - 1) * DAY_MS;
  return records30.value.filter((r) => r.ts >= cutoff);
});

/** Providers present in the current date window, for the checklist. */
const providers = computed<{ key: string; label: string }[]>(() =>
  [...new Set(windowRecords.value.map(providerKeyOf))]
    .map((key) => ({ key, label: providerLabels.value.get(key) ?? key }))
    .sort((a, b) => a.label.localeCompare(b.label)),
);

function toggleProvider(key: string) {
  const next = new Set(excludedProviders.value);
  if (next.has(key)) next.delete(key);
  else next.add(key);
  excludedProviders.value = next;
}
function includeAllProviders() {
  excludedProviders.value = new Set();
}

const dateRecords = computed<UsageRecord[]>(() =>
  windowRecords.value.filter((r) => !excludedProviders.value.has(providerKeyOf(r))),
);

// ── Date view: daily buckets ──
interface DayBucket {
  label: string;
  tokens: number;
  requests: number;
}

const dailyBuckets = computed<DayBucket[]>(() => {
  const today = startOfDay(Date.now());
  const first = today - (days.value - 1) * DAY_MS;
  const byDay = new Map<number, DayBucket>();
  const buckets: DayBucket[] = [];
  for (let i = 0; i < days.value; i++) {
    const dayStart = first + i * DAY_MS;
    const d = new Date(dayStart);
    const bucket: DayBucket = {
      label: `${d.getMonth() + 1}/${d.getDate()}`,
      tokens: 0,
      requests: 0,
    };
    buckets.push(bucket);
    byDay.set(dayStart, bucket);
  }
  for (const r of dateRecords.value) {
    const bucket = byDay.get(startOfDay(r.ts));
    if (!bucket) continue;
    bucket.requests += 1;
    bucket.tokens += r.total ?? 0;
  }
  return buckets;
});

const dateTotals = computed(() => ({
  tokens: dateRecords.value.reduce((s, r) => s + (r.total ?? 0), 0),
  requests: dateRecords.value.length,
}));

/** 7-day window labels every day; longer windows label ~6 ticks. */
const labelStep = computed(() => (days.value <= 7 ? 1 : Math.ceil(days.value / 6)));
function showBarLabel(index: number): boolean {
  return index % labelStep.value === 0 || index === days.value - 1;
}

function barHeight(value: number, max: number): string {
  if (value <= 0 || max <= 0) return "0%";
  return `${Math.max((value / max) * 100, 2.5)}%`;
}

// ── Mode / model views: donut groups (fixed 30-day window) ──
interface Group {
  key: string;
  label: string;
  sub?: string;
  tokens: number;
  requests: number;
}

function groupBy(
  records: UsageRecord[],
  keyFn: (r: UsageRecord) => { key: string; label: string; sub?: string },
): Group[] {
  const map = new Map<string, Group>();
  for (const r of records) {
    const { key, label, sub } = keyFn(r);
    let g = map.get(key);
    if (!g) {
      g = { key, label, sub, tokens: 0, requests: 0 };
      map.set(key, g);
    }
    g.requests += 1;
    g.tokens += r.total ?? 0;
  }
  return [...map.values()].sort((a, b) => b.tokens - a.tokens || b.requests - a.requests);
}

/** Keep the legend readable: top 8 slices, the tail folds into "Other". */
function capGroups(groups: Group[], max = 8): Group[] {
  if (groups.length <= max) return groups;
  const head = groups.slice(0, max);
  const tail = groups.slice(max);
  head.push({
    key: "__other__",
    label: t("settings.usageStats.other"),
    tokens: tail.reduce((s, g) => s + g.tokens, 0),
    requests: tail.reduce((s, g) => s + g.requests, 0),
  });
  return head;
}

function modeLabel(mode: string): string {
  const def = MODES.find((m) => m.id === mode);
  return def ? t(def.labelKey) : mode;
}

const modeGroups = computed(() =>
  capGroups(groupBy(records30.value, (r) => ({ key: r.mode, label: modeLabel(r.mode) }))),
);
const modelGroups = computed(() =>
  // Group by provider identity + model ID: the same model ID served by two
  // different providers is two separate rows, each annotated with its
  // provider on the second line of the legend label.
  capGroups(
    groupBy(records30.value, (r) => {
      const key = providerKeyOf(r);
      return {
        key: `${key} · ${r.model}`,
        label: r.model,
        sub: providerLabels.value.get(key) ?? r.provider,
      };
    }),
  ),
);

// ── Donut geometry ──
// Each view renders two donuts — token share and request share — mirroring
// the two bar charts of the date view.
const DONUT_R = 56;
const DONUT_C = 2 * Math.PI * DONUT_R;
const SEGMENT_OPACITIES = [1, 0.72, 0.55, 0.42, 0.32, 0.25];

interface DonutSeg {
  group: Group;
  pct: number;
  dash: number;
  offset: number;
  opacity: number;
}
type DonutMetric = "tokens" | "requests";

function buildDonut(
  groups: Group[],
  metric: DonutMetric,
): { total: number; segments: DonutSeg[] } {
  const valueOf = (g: Group) => (metric === "tokens" ? g.tokens : g.requests);
  const total = groups.reduce((s, g) => s + valueOf(g), 0);
  let acc = 0;
  const segments = groups.map((g, i) => {
    const pct = total > 0 ? valueOf(g) / total : 0;
    const seg: DonutSeg = {
      group: g,
      pct,
      dash: pct * DONUT_C,
      offset: acc * DONUT_C,
      opacity: SEGMENT_OPACITIES[i % SEGMENT_OPACITIES.length],
    };
    acc += pct;
    return seg;
  });
  return { total, segments };
}

/** The token donut is dropped when no provider in the window reported usage
 *  (a ring of zeros says nothing). */
const donutCards = computed(() => {
  const groups = view.value === "mode" ? modeGroups.value : modelGroups.value;
  const cards = [
    { kind: "tokens" as DonutMetric, donut: buildDonut(groups, "tokens") },
    { kind: "requests" as DonutMetric, donut: buildDonut(groups, "requests") },
  ];
  return cards.filter((c) => c.kind !== "tokens" || c.donut.total > 0);
});

/** Format a metric value for donut cards / bar tooltips. `full` swaps the
 *  compact formatter for the exact number (hover precision). */
function metricText(kind: DonutMetric, value: number, full = false): string {
  return kind === "tokens"
    ? `${(full ? fmtFull : fmt)(value)} tokens`
    : t("settings.usageStats.requests", value);
}

// ── Formatting ──
const compactFmt = computed(
  () => new Intl.NumberFormat(locale.value === "auto" ? undefined : locale.value, {
    notation: "compact",
    maximumFractionDigits: 1,
  }),
);
const fullFmt = computed(
  () => new Intl.NumberFormat(locale.value === "auto" ? undefined : locale.value),
);
const fmt = (n: number) => compactFmt.value.format(n);
const fmtFull = (n: number) => fullFmt.value.format(n);

function pctText(pct: number): string {
  if (pct > 0 && pct < 0.01) return "<1%";
  return `${Math.round(pct * 100)}%`;
}

async function handleDrag(e: MouseEvent) {
  const target = e.target as HTMLElement;
  if (target.closest("button, input, textarea, a, select")) return;
  await getCurrentWindow().startDragging();
}
</script>

<template>
  <div class="ud-root" :class="{ 'grow-above': growAbove }" @mousedown="handleDrag">
    <!-- Header -->
    <div class="ud-header">
      <button class="back-btn" @click="router.push('/settings?tab=general')">
        <ArrowLeft :size="16" />
      </button>
      <span class="header-title">{{ t('settings.usageStats.pageTitle') }}</span>
    </div>

    <!-- Body -->
    <div class="ud-body">
      <p class="ud-hint usage-hint">
        <Info :size="12" :stroke-width="1.9" />
        {{ t('settings.usageStats.disclaimer') }}
      </p>

      <!-- View switcher -->
      <div class="usage-seg">
        <button :class="{ active: view === 'date' }" @click="view = 'date'">
          <CalendarDays :size="12" :stroke-width="1.9" />{{ t('settings.usageStats.viewDate') }}
        </button>
        <button :class="{ active: view === 'mode' }" @click="view = 'mode'">
          <PieChart :size="12" :stroke-width="1.9" />{{ t('settings.usageStats.viewMode') }}
        </button>
        <button :class="{ active: view === 'model' }" @click="view = 'model'">
          <Cpu :size="12" :stroke-width="1.9" />{{ t('settings.usageStats.viewModel') }}
        </button>
      </div>

      <div v-if="records30.length === 0" class="usage-empty">
        {{ t('settings.usageStats.empty') }}
      </div>

      <!-- ─── By date ─── -->
      <template v-else-if="view === 'date'">
        <div class="usage-seg seg-small">
          <button :class="{ active: days === 7 }" @click="days = 7">{{ t('settings.usageStats.last7Days') }}</button>
          <button :class="{ active: days === 30 }" @click="days = 30">{{ t('settings.usageStats.last30Days') }}</button>
        </div>

        <div v-if="providers.length > 1" class="usage-chips">
          <button
            class="usage-chip check"
            :class="{ active: excludedProviders.size === 0 }"
            @click="includeAllProviders"
          >
            <span class="chip-box"><Check v-if="excludedProviders.size === 0" :size="9" :stroke-width="3" /></span>
            {{ t('settings.usageStats.allProviders') }}
          </button>
          <button
            v-for="p in providers"
            :key="p.key"
            class="usage-chip check"
            :class="{ active: !excludedProviders.has(p.key) }"
            @click="toggleProvider(p.key)"
          >
            <span class="chip-box"><Check v-if="!excludedProviders.has(p.key)" :size="9" :stroke-width="3" /></span>
            {{ p.label }}
          </button>
        </div>

        <div v-if="dateRecords.length === 0" class="usage-empty">
          {{ t('settings.usageStats.empty') }}
        </div>
        <template v-else>
          <!-- Daily tokens / daily requests — same card, two metrics -->
          <div v-for="kind in (['tokens', 'requests'] as const)" :key="kind" class="chart-card">
            <div class="chart-head">
              <span class="chart-title">{{ kind === 'tokens' ? t('settings.usageStats.dailyTokens') : t('settings.usageStats.dailyRequests') }}</span>
              <span class="chart-total">{{ t('settings.usageStats.total', { n: metricText(kind, dateTotals[kind]) }) }}</span>
            </div>
            <div class="bars-row" :key="`${kind}-${days}`">
              <div v-for="(b, i) in dailyBuckets" :key="i" class="bar-cell">
                <div
                  class="bar"
                  :style="{ height: barHeight(b[kind], Math.max(...dailyBuckets.map(x => x[kind]))) }"
                  :title="`${b.label} · ${metricText(kind, b[kind], true)}`"
                />
              </div>
            </div>
            <div class="labels-row">
              <span v-for="(b, i) in dailyBuckets" :key="i" class="bar-label">{{ showBarLabel(i) ? b.label : '' }}</span>
            </div>
          </div>
        </template>
      </template>

      <!-- ─── By mode / by model (token + request donuts) ─── -->
      <template v-else>
        <div v-for="card in donutCards" :key="`${view}-${card.kind}`" class="chart-card">
          <div class="chart-head">
            <span class="chart-title">
              {{ card.kind === 'tokens' ? t('settings.usageStats.tokensShare') : t('settings.usageStats.requestsShare') }}
            </span>
            <span class="chart-total">
              {{ t('settings.usageStats.total', { n: metricText(card.kind, card.donut.total) }) }}
            </span>
          </div>
          <div class="donut-layout">
            <div class="donut-wrap">
              <svg viewBox="0 0 140 140" class="donut-svg">
                <circle cx="70" cy="70" :r="DONUT_R" fill="none" class="donut-track" :stroke-width="18" />
                <circle
                  v-for="seg in card.donut.segments"
                  :key="seg.group.key"
                  cx="70" cy="70" :r="DONUT_R" fill="none"
                  class="donut-seg"
                  :stroke-opacity="seg.opacity"
                  :stroke-width="18"
                  :stroke-dasharray="`${seg.dash} ${DONUT_C - seg.dash}`"
                  :stroke-dashoffset="`${-seg.offset}`"
                  transform="rotate(-90 70 70)"
                />
                <text x="70" y="67" text-anchor="middle" class="donut-total">{{ fmt(card.donut.total) }}</text>
                <text x="70" y="82" text-anchor="middle" class="donut-metric">
                  {{ card.kind === 'tokens' ? 'tokens' : t('settings.usageStats.requestsMetric') }}
                </text>
              </svg>
            </div>
            <div class="legend">
              <div v-for="seg in card.donut.segments" :key="seg.group.key" class="legend-row">
                <span class="legend-dot" :style="{ opacity: seg.opacity }" />
                <span class="legend-name">
                  <span class="legend-primary">{{ seg.group.label }}</span>
                  <span v-if="seg.group.sub" class="legend-sub">{{ seg.group.sub }}</span>
                </span>
                <span class="legend-pct">{{ pctText(seg.pct) }}</span>
                <span class="legend-stats">{{ metricText(card.kind, seg.group[card.kind]) }}</span>
              </div>
            </div>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.usage-hint {
  display: flex;
  align-items: center;
  gap: 5px;
  margin: 0;
}
.usage-hint svg { flex-shrink: 0; }

/* Segmented control (view switcher + 7/30-day toggle) */
.usage-seg {
  display: flex;
  gap: 2px;
  padding: 2px;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  flex-shrink: 0;
}
.usage-seg button {
  flex: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  padding: 5px 10px;
  border: none;
  border-radius: var(--radius-sm);
  background: none;
  cursor: pointer;
  font-size: var(--text-sm);
  font-weight: var(--weight-medium);
  color: var(--color-text-muted);
  transition: 0.12s;
  white-space: nowrap;
}
.usage-seg button:hover { color: var(--color-text-secondary); }
.usage-seg button.active {
  background: var(--color-surface-hover);
  color: var(--color-text);
  font-weight: var(--weight-semibold);
}
.usage-seg.seg-small { align-self: flex-start; }
.usage-seg.seg-small button { flex: none; padding: 4px 10px; }

/* Provider drill-down chips */
.usage-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.usage-chip {
  padding: 3px 10px;
  border-radius: var(--radius-pill);
  border: 1px solid var(--color-border);
  background: var(--color-surface);
  color: var(--color-text-muted);
  font-size: var(--text-xs);
  font-weight: var(--weight-medium);
  cursor: pointer;
  transition: 0.12s;
}
.usage-chip:hover { color: var(--color-text-secondary); border-color: var(--color-border-hover); }
.usage-chip.active {
  color: var(--color-accent-text);
  background: var(--color-accent-bg);
  border-color: var(--color-accent-border);
}
.usage-chip.check {
  display: inline-flex;
  align-items: center;
  gap: 5px;
}
.chip-box {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 11px;
  height: 11px;
  border: 1px solid currentColor;
  border-radius: 3px;
  flex-shrink: 0;
  opacity: 0.75;
}

/* Cards */
.chart-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: var(--space-3) var(--space-4);
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}
.chart-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: var(--space-2);
}
.chart-title {
  font-size: var(--text-sm);
  font-weight: var(--weight-semibold);
  color: var(--color-text-secondary);
}
.chart-total {
  font-size: var(--text-xs);
  font-weight: var(--weight-medium);
  color: var(--color-text-muted);
  font-variant-numeric: tabular-nums;
}

/* Bar charts — bars and labels in separate rows so the x-axis is one
   continuous, always-level line across the card. */
.bars-row {
  display: flex;
  gap: 3px;
  height: 84px;
  border-bottom: 1px solid var(--color-border);
}
.bar-cell {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: flex-end;
}
.bar {
  width: 100%;
  background: var(--color-accent);
  border-radius: 2px 2px 0 0;
  min-height: 0;
  transition: height 0.15s ease;
  animation: bar-grow 0.25s ease;
}
/* Mount grow-in: on a 7↔30-day switch the whole bars row remounts (keyed by
   `days`), so every bar grows from zero at its correct position instead of
   the old index-keyed bars morphing at positions whose dates have changed. */
@keyframes bar-grow {
  from { height: 0; }
}
.bar:hover { filter: brightness(1.15); }
.labels-row {
  display: flex;
  gap: 3px;
  margin-top: 3px;
}
.bar-label {
  flex: 1;
  min-width: 0;
  font-size: 9px;
  color: var(--color-text-muted);
  text-align: center;
  white-space: nowrap;
  overflow: visible;
  font-variant-numeric: tabular-nums;
}
@media (prefers-reduced-motion: reduce) {
  .bar { transition: none; animation: none; }
}

/* Donut */
.donut-layout {
  display: flex;
  align-items: center;
  gap: var(--space-4);
}
.donut-wrap { flex-shrink: 0; width: 120px; }
.donut-svg { width: 120px; height: 120px; display: block; }
.donut-track { stroke: var(--color-surface-hover); }
.donut-seg { stroke: var(--color-accent); }
.donut-total {
  fill: var(--color-text);
  font-size: 16px;
  font-weight: var(--weight-bold);
  font-variant-numeric: tabular-nums;
}
.donut-metric {
  fill: var(--color-text-muted);
  font-size: 9px;
  font-weight: var(--weight-medium);
}

/* Legend */
.legend {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.legend-row {
  display: flex;
  align-items: center;
  gap: 7px;
  font-size: var(--text-sm);
}
.legend-dot {
  width: 8px;
  height: 8px;
  border-radius: 2px;
  background: var(--color-accent);
  flex-shrink: 0;
}
.legend-name {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}
.legend-primary {
  color: var(--color-text);
  font-weight: var(--weight-medium);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.legend-sub {
  color: var(--color-text-muted);
  font-size: var(--text-xs);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.legend-pct {
  color: var(--color-text-secondary);
  font-weight: var(--weight-semibold);
  font-variant-numeric: tabular-nums;
  flex-shrink: 0;
}
.legend-stats {
  color: var(--color-text-muted);
  font-size: var(--text-xs);
  font-variant-numeric: tabular-nums;
  flex-shrink: 0;
  min-width: 110px;
  text-align: right;
}

.usage-empty {
  padding: var(--space-6) 0;
  text-align: center;
  font-size: var(--text-sm);
  color: var(--color-text-muted);
}
</style>
