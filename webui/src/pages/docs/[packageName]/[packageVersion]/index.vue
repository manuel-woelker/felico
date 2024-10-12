<script setup lang="ts">
import {usePackageStore} from '@/stores/PackageStore'
import {FlowerSpinner} from 'epic-spinners'
import {storeToRefs} from 'pinia'

const packageStore = usePackageStore();
const {pkg} = storeToRefs(packageStore)
</script>

<template>
  <div>Package Docs</div>
  <RouterLink to="/docs/std/1.2.3">std 1.2.3</RouterLink>
  <div v-if="pkg">
    <div>
      <h2> {{ pkg.info.name }} </h2>
      {{ pkg.info.version }}
    </div>
    <div>
    </div>
  </div>
  <div v-else>
    <div>
      <flower-spinner
          :animation-duration="1000"
          :color="'rgba(0,73,31,0.35)'"
          :size="60"
      />
      Loading...
    </div>
  </div>
</template>

<style scoped lang="sass">

</style>
