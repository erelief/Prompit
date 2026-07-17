/**
 * The single source of truth for the data categories that backup / restore /
 * reset can act on. The Rust side (`vault.rs` `KNOWN_STEMS`) must agree on the
 * stem strings; this module mirrors those strings for the frontend.
 *
 * `settings` corresponds to the otherwise-plaintext `config.json` (software
 * settings, minus its `webdav` key) and `webdav` to the WebDAV server
 * settings split out of it — those two are the newest categories.
 */

export type DataCategory =
  | "settings"
  | "webdav"
  | "providers"
  | "websearch"
  | "history"
  | "dictionaries"
  | "personas"
  | "skills_lite";

export interface CategoryMeta {
  /** i18n key under `settings.categories.<id>` for the human label. */
  labelKey: string;
  /** i18n key under `settings.categories.<id>Description` for the description. */
  descKey: string;
  /** Carries api keys / credentials. Used to flag sensitive rows. */
  sensitive: boolean;
}

export const ALL_CATEGORIES: DataCategory[] = [
  "providers",
  "websearch",
  "personas",
  "dictionaries",
  "skills_lite",
  "settings",
  "webdav",
  "history",
];

/**
 * Default selection for backup / restore: everything except software settings,
 * WebDAV server settings and history (those are less likely to need backing up
 * and more likely to differ between machines). Reset always selects every
 * category.
 */
export function defaultSelectedCategories(
  mode: "export" | "import" | "reset",
): DataCategory[] {
  if (mode === "reset") return [...ALL_CATEGORIES];
  return ALL_CATEGORIES.filter(
    (c) => c !== "settings" && c !== "history" && c !== "webdav",
  );
}

export const CATEGORY_META: Record<DataCategory, CategoryMeta> = {
  settings: {
    labelKey: "settings.categories.settings",
    descKey: "settings.categories.settingsDescription",
    sensitive: false,
  },
  webdav: {
    labelKey: "settings.categories.webdav",
    descKey: "settings.categories.webdavDescription",
    sensitive: false,
  },
  providers: {
    labelKey: "settings.categories.providers",
    descKey: "settings.categories.providersDescription",
    sensitive: true,
  },
  websearch: {
    labelKey: "settings.categories.websearch",
    descKey: "settings.categories.websearchDescription",
    sensitive: true,
  },
  history: {
    labelKey: "settings.categories.history",
    descKey: "settings.categories.historyDescription",
    sensitive: false,
  },
  dictionaries: {
    labelKey: "settings.categories.dictionaries",
    descKey: "settings.categories.dictionariesDescription",
    sensitive: false,
  },
  personas: {
    labelKey: "settings.categories.personas",
    descKey: "settings.categories.personasDescription",
    sensitive: false,
  },
  skills_lite: {
    labelKey: "settings.categories.skills_lite",
    descKey: "settings.categories.skills_liteDescription",
    sensitive: false,
  },
};

/** Preview entry returned by `inspect_bundle` for a single category. */
export interface CategoryPreview {
  id: string;
  /** `n` for the list-shaped categories (providers/websearch), else null. */
  count: number | null;
}

export interface BundlePreview {
  version: number;
  categories: CategoryPreview[];
}

/**
 * Filter a bundle preview down to the categories this app actually knows
 * about (drops any unknown ids a future/newer bundle might carry). Shared by
 * the import UIs so the filter rule lives in one place.
 */
export function knownCategoriesIn(preview: CategoryPreview[]): DataCategory[] {
  // Preserve the canonical ALL_CATEGORIES order so every import UI lists rows
  // in the same sequence as export / reset, regardless of how the backend
  // ordered the bundle (KNOWN_STEMS). Only categories actually present in the
  // bundle are kept; unknown ids are dropped.
  const present = new Set(preview.map((c) => c.id));
  return ALL_CATEGORIES.filter((c) => present.has(c));
}
