<template>
  <svg
    :width="size"
    :height="size"
    viewBox="5.7 7.09 18.6 15.82"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
  >
    <path
      d="M15.47,7.1L14.17,8.95C13.97,9.24 13.63,9.42 13.27,9.42L6.17,9.42L6.17,7.09C6.16,7.1 15.47,7.1 15.47,7.1Z"
      fill="currentColor"
    />
    <path
      d="M24.3,7.1L13.14,22.91L5.7,22.91L16.86,7.1L24.3,7.1Z"
      fill="currentColor"
    />
    <path
      d="M14.53,22.91L15.84,21.05C16.04,20.76 16.38,20.58 16.74,20.58L23.83,20.58L23.83,22.91L14.53,22.91Z"
      fill="currentColor"
    />
  </svg>
</template>

<script setup lang="ts">
defineProps<{ size?: number }>();
</script>
