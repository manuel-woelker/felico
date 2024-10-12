import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { delay } from '@/util/delay'

export const usePackageStore = defineStore("packageIndex", () =>{
  const route = useRoute();
  const pkg = ref(null);
  watch(() => [route.params.packageName, route.params.packageVersion], getPackage, { immediate: true })

  async function getPackage([packageName, packageVersion]: [string, string]) {
    console.log(route.params);
    pkg.value = null;

    try {
      await delay(1000);
      const response = await fetch(`/api/packages/${packageName}/${packageVersion}`);
      const packageDescription = (await response.json()) as PackageIndex
      pkg.value = packageDescription
      console.log(packageDescription);
//      packages.value = packageIndexJson.packages;
    } catch (error) {
      console.error(error)
    }
  }


  return {pkg};
});
