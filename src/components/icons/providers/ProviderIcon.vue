<template>
  <Cloud v-if="!iconComponent" :size="size" :stroke-width="1.5" />
  <component v-else :is="iconComponent" :size="size" />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Cloud } from '@lucide/vue'
import { PROVIDER_ICONS } from './index'

const props = withDefaults(defineProps<{
  icon: string
  size?: number
}>(), {
  size: 16,
})

const iconComponent = computed(() => props.icon ? PROVIDER_ICONS[props.icon] : undefined)
</script>
