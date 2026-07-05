/**
 * The single source of truth for the data categories that import / export /
 * reset can act on. The Rust side (`vault.rs` `KNOWN_STEMS`) must agree on the
 * stem strings; this module mirrors those strings for the frontend.
 *
 * `settings` corresponds to the otherwise-plaintext `config.json` (software
 * settings) and is the newest category. `secrets` is a legacy/orphaned store
 * kept for migration; it is shown but de-emphasized.
 */

export type DataCategory =
  | "settings"
  | "providers"
  | "websearch"
  | "history"
  | "dictionaries"
  | "personas"
  | "skills_lite"
  | "secrets";

export interface CategoryMeta {
  /** i18n key under `settings.categories.<id>` for the human label. */
  labelKey: string;
  /** i18n key under `settings.categories.<id>Description` for the description. */
  descKey: string;
  /** Carries api keys / credentials. Used to flag sensitive rows. */
  sensitive: boolean;
  /** Legacy / orphaned store; rendered with a muted badge, default-on but quiet. */
  legacy?: boolean;
}

export const ALL_CATEGORIES: DataCategory[] = [
  "settings",
  "providers",
  "websearch",
  "history",
  "dictionaries",
  "personas",
  "skills_lite",
  "secrets",
];

export const CATEGORY_META: Record<DataCategory, CategoryMeta> = {
  settings: {
    labelKey: "settings.categories.settings",
    descKey: "settings.categories.settingsDescription",
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
  secrets: {
    labelKey: "settings.categories.secrets",
    descKey: "settings.categories.secretsDescription",
    sensitive: true,
    legacy: true,
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
