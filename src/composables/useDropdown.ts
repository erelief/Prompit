import { ref, nextTick, type Ref } from "vue";

export interface UseDropdownOptions {
  offset?: number;
  posOverride?: (btnRect: DOMRect) => { top: number; left: number };
  siblings?: Ref<ReturnType<typeof useDropdown>[]>;
}

export function useDropdown(opts: UseDropdownOptions = {}) {
  const { offset = 4, posOverride, siblings = ref([]) } = opts;

  const show = ref(false);
  const openedAbove = ref(false);
  const containerRef = ref<HTMLElement | null>(null);
  const btnRef = ref<HTMLElement | null>(null);
  const menuRef = ref<HTMLElement | null>(null);
  const pos = ref({ top: 0, left: 0 });

  const instance = {
    show, openedAbove, containerRef, btnRef, menuRef, pos,
    toggle, close, chevronTransform,
  };

  function close() {
    show.value = false;
  }

  function toggle() {
    for (const sib of siblings.value) {
      if (sib !== instance) sib.close();
    }
    if (!show.value && btnRef.value) {
      const rect = btnRef.value.getBoundingClientRect();
      const p = posOverride ? posOverride(rect) : { top: rect.bottom + offset, left: rect.left };
      pos.value = p;
      show.value = true;
      nextTick(() => {
        if (menuRef.value) {
          const menuH = menuRef.value.offsetHeight;
          const below = window.innerHeight - rect.bottom - offset;
          const above = rect.top - offset;
          if (menuH > below && menuH <= above) {
            pos.value = { ...p, top: rect.top - menuH - offset };
            openedAbove.value = true;
          } else {
            openedAbove.value = false;
          }
        }
      });
    } else {
      show.value = false;
    }
  }

  function chevronTransform() {
    return `rotate(${show.value === openedAbove.value ? 0 : 180}deg)`;
  }

  return instance;
}
