<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 22 22"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
  >
    <g transform="matrix(1.098496,0,0,1.134329,-1.235804,-3.83583)">
      <path
        d="M4.66,15.82C7.36,10.23 12.6,5.85 17.62,4.19L15.06,17.64L4.66,15.82ZM7.59,14.48L13.52,15.52L15.1,7.36C12.37,8.69 9.28,11.8 7.59,14.48Z"
        fill="currentColor"
      />
    </g>
    <g transform="matrix(-1,0,-0,-0.950651,24.558883,40.670963)">
      <path
        d="M17.15,22.78L8.06,22.78L8.41,20.61L16.81,20.61L17.15,22.78Z"
        fill="currentColor"
      />
    </g>
  </svg>
</template>

<script setup lang="ts">
defineProps<{ size?: number }>();
</script>
