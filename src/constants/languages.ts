import i18n from "../i18n";

export const BUILTIN_LANGUAGES: string[] = [
  "English", "Japanese", "Korean", "French", "German", "Spanish", "Russian",
  "Simplified Chinese", "Traditional Chinese",
];

export const LANG_CODE_MAP: Record<string, string> = {
  "English": "EN",
  "Simplified Chinese": "SC",
  "Traditional Chinese": "TC",
  "Japanese": "JA",
  "Korean": "KO",
  "French": "FR",
  "German": "DE",
  "Spanish": "ES",
  "Russian": "RU",
};

export const LANGUAGE_GROUPS: Record<string, string[]> = {
  "English": ["English"],
  "Japanese": ["Japanese"],
  "Korean": ["Korean"],
  "French": ["French"],
  "German": ["German"],
  "Spanish": ["Spanish"],
  "Russian": ["Russian"],
  "Simplified Chinese": ["Simplified Chinese", "Traditional Chinese"],
};

export function getLangName(lang: string): string {
  const entry = i18n.global.tm(`languages.${lang}`) as Record<string, string> | undefined;
  if (entry && typeof entry === "object" && "name" in entry) return entry.name;
  return lang;
}

export function getLangCode(lang: string): string {
  const entry = i18n.global.tm(`languages.${lang}`) as Record<string, string> | undefined;
  if (entry && typeof entry === "object" && "code" in entry) return entry.code;
  return LANG_CODE_MAP[lang] || lang.slice(0, 2).toUpperCase();
}
