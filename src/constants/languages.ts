export const BUILTIN_LANGUAGES: string[] = [
  "English", "Japanese", "Korean", "French", "German", "Spanish", "Russian",
  "Simplified Chinese", "Traditional Chinese",
];

export const LANG_CODE_MAP: Record<string, string> = {
  "English": "EN",
  "Simplified Chinese": "简中",
  "Traditional Chinese": "繁中",
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
