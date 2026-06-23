<template>
  <svg :width="size" :height="size" viewBox="0 0 24 24" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
    <!-- AnySearch brand mark (stylized "A"). Source: assets/.sourcefiles/AnySearch.svg.
         Monochrome via currentColor; two tones preserved with fill-opacity. -->
    <g transform="matrix(1,0,0,1.0311,-0.057145,-0.290739)">
      <rect x="0.927" y="0.662" width="5.824" height="22.008" />
    </g>
    <g transform="matrix(0.847933,0,0,0.253444,17.283841,17.338482)">
      <rect x="0.927" y="0.662" width="5.824" height="22.008" />
    </g>
    <g transform="matrix(0.976522,0,0,0.253444,5.789122,17.338482)">
      <rect x="0.927" y="0.662" width="5.824" height="22.008" style="fill-opacity:0.5;" />
    </g>
    <g transform="matrix(0.847933,0,0,0.254742,17.283841,0.223338)">
      <rect x="0.927" y="0.662" width="5.824" height="22.008" />
    </g>
    <g transform="matrix(0.847933,0,0,0.265442,17.283841,5.822503)">
      <rect x="0.927" y="0.662" width="5.824" height="22.008" style="fill-opacity:0.5;" />
    </g>
    <g transform="matrix(1.953044,0,0,0.254742,4.883767,0.223338)">
      <rect x="0.927" y="0.662" width="5.824" height="22.008" style="fill-opacity:0.5;" />
    </g>
  </svg>
</template>

<script setup lang="ts">
defineProps<{ size?: number }>();
</script>
