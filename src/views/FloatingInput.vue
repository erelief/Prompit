<script setup lang="ts">
import { ref, nextTick, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useShortcutTriggered } from "../composables/useTauriEvents";
import { loadConfig } from "../stores/config";
import { translate } from "../services/llm-client";

const inputText = ref("");
const translatedText = ref("");
const isLoading = ref(false);
const errorMessage = ref("");
const textareaRef = ref<HTMLTextAreaElement | null>(null);

function handleKeydown(e: KeyboardEvent) {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    handleTranslate();
  }
  if (e.key === "Escape") {
    handleHide();
  }
}

async function handleTranslate() {
  const text = inputText.value.trim();
  if (!text || isLoading.value) return;

  errorMessage.value = "";
  translatedText.value = "";
  isLoading.value = true;

  try {
    const result = await translate(text);
    translatedText.value = result;

    await invoke("hide_main_window");
    await invoke("simulate_paste", { text: result });
  } catch (err) {
    errorMessage.value = String(err);
  } finally {
    isLoading.value = false;
  }
}

async function handleHide() {
  await invoke("hide_main_window");
}

async function handleOpenSettings() {
  await invoke("open_settings_window");
}

function clearAll() {
  inputText.value = "";
  translatedText.value = "";
  errorMessage.value = "";
}

onMounted(async () => {
  await loadConfig();
  nextTick(() => {
    textareaRef.value?.focus();
  });
});

defineExpose({ clearAll });

useShortcutTriggered(() => {
  clearAll();
  nextTick(() => {
    textareaRef.value?.focus();
  });
});
</script>

<template>
  <div
    class="w-full h-full flex items-center justify-center bg-black/60 backdrop-blur-sm rounded-xl"
  >
    <div class="w-full max-w-[580px] px-4 py-3 flex flex-col gap-2">
      <!-- Input area -->
      <div class="flex items-end gap-2">
        <textarea
          ref="textareaRef"
          v-model="inputText"
          @keydown="handleKeydown"
          placeholder="Type text to translate... (Enter to translate, Esc to hide)"
          rows="1"
          class="flex-1 resize-none bg-white/10 text-white placeholder:text-white/40
                 rounded-lg px-3 py-2 text-sm outline-none border border-white/20
                 focus:border-white/40 max-h-[120px] overflow-y-auto"
          style="field-sizing: content"
        />
        <button
          @click="handleOpenSettings"
          class="shrink-0 w-8 h-8 flex items-center justify-center
                 text-white/50 hover:text-white/80 transition-colors"
          title="Settings"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <circle cx="12" cy="12" r="3" />
            <path
              d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1
              0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0
              0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2
              2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65
              1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2
              0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68
              15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2
              0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65
              0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2
              0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9
              4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2
              2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65
              1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0
              2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0
              19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2
              2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"
            />
          </svg>
        </button>
      </div>

      <!-- Status area -->
      <div v-if="isLoading" class="text-white/60 text-xs">Translating...</div>
      <div v-if="errorMessage" class="text-red-400 text-xs">
        {{ errorMessage }}
      </div>
      <div
        v-if="translatedText"
        class="text-white/90 text-sm bg-white/5 rounded-lg px-3 py-2"
      >
        {{ translatedText }}
      </div>
    </div>
  </div>
</template>
