<script setup lang="ts">
defineProps<{ size?: number; strokeWidth?: number }>();
</script>

<template>
  <svg
    :width="size ?? 11"
    :height="size ?? 11"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    :stroke-width="strokeWidth ?? 2"
    stroke-linecap="round"
    stroke-linejoin="round"
  >
    <path d="M2,10L2,13" />
    <path d="M6,6L6,17" />
    <path d="M10,3L10,21" />
    <path d="M14,8L14,15" />
    <path d="M18,5C18,5 17.986,8.85 17.986,13.183" />
    <path d="M22,10L22,13" />
    <path d="M14,19.5L17,16.5L20,19.5" />
    <path d="M17,22L17,16.5" />
  </svg>
</template>
