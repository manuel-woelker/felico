import {createStore} from "solid-js/store"

export interface BundleIndex {
  bundles: BundleInfo[]
}

export interface BundleInfo {
  name: string
  version: string
}


// Initialize store
export const [bundleIndexStore, setBundleIndexStore] = createStore<BundleIndex>({
  bundles: [],
});

document.addEventListener("DOMContentLoaded", async () => {
  console.log("load");
  const response = await fetch('/api/bundles')
  const bundleIndex = (await response.json());
  console.log(bundleIndex);
  setBundleIndexStore(bundleIndex);
});

