<script setup lang="ts">
import { useI18n } from "vue-i18n";
import LicenseText from "../components/LicenseText.vue";
// Vite resolves ?raw imports to the file's verbatim text content at build
// time, so the license text is bundled into the app and always in sync with
// the project's LICENSE file. The path is relative to the vite config root
// (the project root), which is where LICENSE lives.
import licenseText from "../../LICENSE?raw";

const { t } = useI18n();
</script>

<template>
  <LicenseText :title="t('about.license')" :text="licenseText" />
</template>
