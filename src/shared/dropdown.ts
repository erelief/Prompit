/**
 * Shared dropdown sizing + chevron helpers, used by FloatingInput and
 * TranslateToolbar (previously duplicated in both files).
 */

/** Row height of a dropdown option, px. */
export const ITEM_H = 28;
/** Vertical padding inside the dropdown container, px. */
export const PAD = 6;

/** Cap a dropdown's max-height to show 2 items, scroll beyond. */
export const capHeight = (n: number) =>
  n > 2 ? { maxHeight: `${2 * ITEM_H + PAD}px` } : {};

/** Chevron rotation for dropdown triggers; flips when the window grows upward. */
export const chevronTransform = (open: boolean, growAbove: boolean) =>
  `rotate(${open === growAbove ? 0 : 180}deg)`;
