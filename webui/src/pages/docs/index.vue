<script setup lang="ts">
import {usePackageIndexStore} from '@/stores/PackageIndexStore'

const packageIndexStore = usePackageIndexStore();
</script>

<template>
  <div>DOCS!</div>
  <RouterLink to="/docs/std/1.2.3">std 1.2.3</RouterLink>

  <div v-for="pkg in packageIndexStore.packages" :key="pkg.name">
    <RouterLink :to="`/docs/${pkg.name}/${pkg.version}`">
      {{ pkg.name }} {{ pkg.version }}
    </RouterLink>
  </div>

</template>

<style scoped lang="sass">

</style>
