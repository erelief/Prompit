<script setup lang="ts">
import { ref, nextTick, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useRouter } from "vue-router";
import { useShortcutTriggered } from "../composables/useTauriEvents";
import { loadConfig } from "../stores/config";
import { translate } from "../services/llm-client";
import { Settings, LoaderCircle, Send, X } from "@lucide/vue";

const router = useRouter();

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
  router.push("/settings");
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
    class="w-full h-full flex items-center justify-center rounded-xl overflow-hidden"
    style="
      background: linear-gradient(
        135deg,
        rgba(15, 15, 20, 0.85) 0%,
        rgba(20, 20, 30, 0.8) 100%
      );
      backdrop-filter: blur(24px) saturate(1.5);
    "
  >
    <div class="w-full max-w-[560px] px-5 py-4 flex flex-col gap-3">
      <!-- Input row -->
      <div class="flex items-end gap-2.5">
        <textarea
          ref="textareaRef"
          v-model="inputText"
          @keydown="handleKeydown"
          placeholder="Type to translate..."
          rows="1"
          class="floating-input flex-1 resize-none text-[13px] leading-relaxed
                 outline-none max-h-[100px] overflow-y-auto"
          style="field-sizing: content"
        />

        <!-- Send button -->
        <button
          @click="handleTranslate"
          :disabled="!inputText.trim() || isLoading"
          class="send-btn shrink-0"
          title="Translate (Enter)"
        >
          <LoaderCircle v-if="isLoading" :size="15" class="animate-spin" />
          <Send v-else :size="15" />
        </button>

        <div class="w-px h-5 bg-white/10 shrink-0 self-center"></div>

        <!-- Settings button -->
        <button
          @click="handleOpenSettings"
          class="icon-btn shrink-0"
          title="Settings"
        >
          <Settings :size="14" :stroke-width="1.8" />
        </button>

        <!-- Dismiss button -->
        <button @click="handleHide" class="icon-btn shrink-0" title="Hide (Esc)">
          <X :size="14" :stroke-width="1.8" />
        </button>
      </div>

      <!-- Status -->
      <Transition name="fade">
        <div
          v-if="isLoading"
          class="flex items-center gap-2 text-[11px] text-white/40"
        >
          <span class="inline-block w-1.5 h-1.5 rounded-full bg-amber-400/60 animate-pulse"></span>
          Translating...
        </div>
      </Transition>

      <Transition name="fade">
        <div
          v-if="errorMessage"
          class="text-[11px] text-red-400/80 flex items-center gap-1.5"
        >
          <X :size="12" :stroke-width="2" />
          {{ errorMessage }}
        </div>
      </Transition>

      <Transition name="fade">
        <div
          v-if="translatedText"
          class="text-[13px] text-white/80 leading-relaxed bg-white/[0.04] rounded-lg px-3.5 py-2.5
                 border border-white/[0.06]"
        >
          {{ translatedText }}
        </div>
      </Transition>
    </div>
  </div>
</template>

<style scoped>
.floating-input {
  background: rgba(255, 255, 255, 0.06);
  color: rgba(255, 255, 255, 0.9);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 10px;
  padding: 9px 14px;
  transition: border-color 0.2s ease, box-shadow 0.2s ease;
}

.floating-input::placeholder {
  color: rgba(255, 255, 255, 0.25);
}

.floating-input:focus {
  border-color: rgba(217, 160, 71, 0.35);
  box-shadow: 0 0 0 2px rgba(217, 160, 71, 0.08);
}

.send-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 34px;
  height: 34px;
  border-radius: 9px;
  background: linear-gradient(135deg, #d4a048 0%, #c4922e 100%);
  color: #1a1a1a;
  transition: all 0.15s ease;
}

.send-btn:hover:not(:disabled) {
  background: linear-gradient(135deg, #ddb35a 0%, #d4a048 100%);
  transform: translateY(-0.5px);
  box-shadow: 0 4px 12px rgba(212, 160, 72, 0.25);
}

.send-btn:active:not(:disabled) {
  transform: translateY(0);
}

.send-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.icon-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border-radius: 8px;
  color: rgba(255, 255, 255, 0.35);
  transition: all 0.15s ease;
}

.icon-btn:hover {
  color: rgba(255, 255, 255, 0.7);
  background: rgba(255, 255, 255, 0.06);
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>
