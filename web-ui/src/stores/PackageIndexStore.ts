import {createStore} from "solid-js/store"

export interface PackageIndex {
  packages: PackageInfo[]
}

export interface PackageInfo {
  name: string
  version: string
}


// Initialize store
export const [packageIndexStore, setPackageIndexStore] = createStore<PackageIndex>({
  packages: [],
});

document.addEventListener("DOMContentLoaded", async () => {
  console.log("load");
  const response = await fetch('/api/packages')
  const packageIndex = (await response.json());
  console.log(packageIndex);
  setPackageIndexStore(packageIndex);
});

