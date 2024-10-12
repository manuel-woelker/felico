<script setup lang="ts">

import { RouterLink } from 'vue-router'
</script>

<template>
  <div class="wrapper">
    <h1>😺 felico</h1>
    <nav>
      <RouterLink to="/">Home</RouterLink>
      <RouterLink to="/docs">Docs</RouterLink>
      <strong>Current route path:</strong> {{ $route.fullPath }}
    </nav>
  </div>

</template>

<style scoped>
  ::root {
    font-size: 20px;
  }
    h1 {
      font-size: 20px;
      margin: 2px;
      padding-left: 10px;
    }
</style>
