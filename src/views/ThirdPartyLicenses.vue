<script setup lang="ts">
import { useI18n } from "vue-i18n";
import LicenseText from "../components/LicenseText.vue";
// Same ?raw pattern as License.vue: bundle the generated file at build time so
// it works offline and under Tauri's CSP without a runtime fetch.
import text from "../../public/THIRD-PARTY-LICENSES?raw";

const { t } = useI18n();
</script>

<template>
  <LicenseText :title="t('about.thirdParty')" :text="text" />
</template>
