<script setup lang="ts">
import { usePackageIndexStore } from '@/stores/PackageIndexStore'

const packageIndexStore = usePackageIndexStore();
</script>

<template>
  <main>
    <div>List</div>
    <div>
      Details
      <div v-for="pkg in packageIndexStore.packages" :key="pkg.name">
        <RouterLink :to="`/docs/${pkg.name}/${pkg.version}`">
        {{ pkg.name }} {{ pkg.version }}
        </RouterLink>
      </div>
      {{packageIndexStore.packages}}

    </div>
  </main>
</template>

<style>
main {
  padding: 10px;
  display: grid;
  grid-template-columns: 150px 1fr;
  grid-template-rows: 100%;
}
</style>
