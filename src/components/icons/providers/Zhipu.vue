<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 30 30"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
  >
    <path
      d="M24.51,28.51L5.49,28.51C3.28,28.51 1.49,26.72 1.49,24.51L1.49,5.49C1.49,3.28 3.28,1.49 5.49,1.49L24.52,1.49C26.73,1.49 28.52,3.28 28.52,5.49L28.52,24.52C28.51,26.72 26.72,28.51 24.51,28.51Z"
      style="
        fill: none;
        fill-rule: nonzero;
        stroke: currentColor;
        stroke-width: 0.63px;
      "
    />
    <path
      d="M15.47,7.1L14.17,8.95C13.97,9.24 13.63,9.42 13.27,9.42L6.17,9.42L6.17,7.09C6.16,7.1 15.47,7.1 15.47,7.1Z"
      style="fill: currentColor; fill-rule: nonzero"
    />
    <path
      d="M24.3,7.1L13.14,22.91L5.7,22.91L16.86,7.1L24.3,7.1Z"
      style="fill: currentColor; fill-rule: nonzero"
    />
    <path
      d="M14.53,22.91L15.84,21.05C16.04,20.76 16.38,20.58 16.74,20.58L23.83,20.58L23.83,22.91L14.53,22.91Z"
      style="fill: currentColor; fill-rule: nonzero"
    />
  </svg>
</template>

<script setup lang="ts">
defineProps<{ size?: number }>();
</script>
