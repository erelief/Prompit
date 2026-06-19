<script setup lang="ts">
import { ImageUp } from "@lucide/vue";
import { useI18n } from "vue-i18n";
import type { ModelInputCapabilities } from "../stores/config";

const props = withDefaults(
  defineProps<{ capabilities?: ModelInputCapabilities; size?: number }>(),
  { size: 11 },
);
const { t } = useI18n();
</script>

<template>
  <ImageUp
    v-if="props.capabilities?.image"
    :size="props.size"
    :stroke-width="1.8"
    :title="t('common.supportsImageInput')"
    style="color: var(--color-text-muted); flex-shrink: 0"
  />
</template>
