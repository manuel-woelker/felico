<template>
  <v-app>
    <v-app-bar color="surface-variant" title="😺 felico">
      <RouterLink to="/">Home</RouterLink>
      <RouterLink to="/docs">Docs</RouterLink>
    </v-app-bar>

    <v-navigation-drawer>
      <v-list>
        <v-list-item title="Drawer left"></v-list-item>
      </v-list>
    </v-navigation-drawer>
    <v-main>
      <router-view/>
    </v-main>

    <AppFooter/>
  </v-app>
</template>

<script lang="ts" setup>
//
</script>
