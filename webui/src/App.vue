<template>
  <v-app>
    <v-main>
      <router-view/>
    </v-main>
  </v-app>
</template>

<script lang="ts" setup>
interface PackageIndex {
  packages: PackageInfo[]
}

interface PackageInfo {
  name: string
  version: string
}

interface ErrorMessage {
  error: string
}

(async function getPackageIndex() {
  try {
    const error_response = await fetch('/api/test_error')
    const error_json = (await error_response.json()) as ErrorMessage
    console.log(error_json)
    console.log(error_json.error)
    const response = await fetch('/api/packages')
    const packageIndex = (await response.json()) as PackageIndex
    console.log(packageIndex)
    packageIndex.packages.forEach(async packageInfo => {
      const response = await fetch(
          `/api/packages/${packageInfo.name}/${packageInfo.version}`,
      )
      const packageDescription = (await response.json()) as PackageIndex
      console.log(packageDescription)
    })
  } catch (error) {
    console.error(error)
  }
})()
</script>
