<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 30 30"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
  >
  >
    <rect
      x="9"
      y="12"
      width="12"
      height="12"
      style="fill: currentColor; fill-opacity: 0.5"
    />
    <path
      d="M27,0L27,30L3,30L3,0L27,0ZM21,6.022L9,6.022L9,24L21,24L21,6.022Z"
      style="fill: currentColor"
    />
  </svg>
</template>

<script setup lang="ts">
defineProps<{ size?: number }>();
</script>
