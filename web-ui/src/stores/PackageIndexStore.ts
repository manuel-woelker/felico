import { defineStore } from 'pinia'
import { ref } from 'vue'

export const usePackageIndexStore = defineStore("packageIndex", () =>{
  const packages = ref(["loading"]);

  (async function getPackageIndex() {
    try {
      const response = await fetch('/api/packages')
      const packageIndexJson = (await response.json()) as PackageIndex
      console.log(packageIndexJson);
      packages.value = packageIndexJson.packages;
    } catch (error) {
      console.error(error)
    }
  })()


  return {packages};
});
