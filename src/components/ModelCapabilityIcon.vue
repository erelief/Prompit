<script setup lang="ts">
import { ImageUp } from "@lucide/vue";
import { useI18n } from "vue-i18n";
import { appConfig } from "../stores/config";
import type { ModelInputCapabilities } from "../stores/config";
import AudioUp from "./icons/AudioUp.vue";
import VideoUp from "./icons/VideoUp.vue";

const props = withDefaults(
  defineProps<{ capabilities?: ModelInputCapabilities; size?: number }>(),
  { size: 11 },
);
const { t } = useI18n();
</script>

<template>
  <template v-if="appConfig.show_capability_icons">
    <ImageUp
      v-if="props.capabilities?.image"
      :size="props.size"
      :stroke-width="1.8"
      :title="t('common.supportsImageInput')"
      style="color: var(--color-text-muted); flex-shrink: 0"
    />
    <AudioUp
      v-if="props.capabilities?.audio"
      :size="props.size"
      :stroke-width="1.8"
      :title="t('common.supportsAudioInput')"
      style="color: var(--color-text-muted); flex-shrink: 0"
    />
    <VideoUp
      v-if="props.capabilities?.video"
      :size="props.size"
      :stroke-width="1.8"
      :title="t('common.supportsVideoInput')"
      style="color: var(--color-text-muted); flex-shrink: 0"
    />
  </template>
</template>
