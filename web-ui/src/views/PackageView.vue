<script setup lang="ts">

import { usePackageStore } from '@/stores/PackageStore'
import { storeToRefs } from 'pinia'
import { AtomSpinner, FlowerSpinner } from 'epic-spinners'

const packageStore = usePackageStore();
const {pkg} = storeToRefs(packageStore)
</script>

<template>
  <main v-if="pkg">
    <div>
      <h2> {{pkg.info.name}} </h2>
      {{pkg.info.version}}
    </div>
    <div>
    </div>
  </main>
  <div v-else>
    <div>
    <flower-spinner
      :animation-duration="1000"
      :size="60"
      :color="'rgba(0,73,31,0.35)'"
    />
    Loading...
    </div>
  </div>
</template>

<style>
main {
  padding: 10px;
  display: grid;
  grid-template-columns: 150px 1fr;
  grid-template-rows: 100%;
}
</style>
