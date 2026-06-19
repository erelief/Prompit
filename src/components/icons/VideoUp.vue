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
    <path d="M11.144,21L5,21C3.896,21 3,20.104 3,19L3,5C3,3.896 3.896,3 5,3L19,3C20.104,3 21,3.896 21,5L21,15.955" />
    <path d="M7,3L7,21" />
    <path d="M3,7.5L7,7.5" />
    <path d="M3,12L21,12" />
    <path d="M3,16.5L7,16.5" />
    <path d="M17,3L17,12.856" />
    <path d="M17,7.5L21,7.5" />
    <path d="M14,19.5L17,16.5L20,19.5" />
    <path d="M17,22L17,16.5" />
  </svg>
</template>
