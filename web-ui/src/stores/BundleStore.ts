import {createResource, createRoot, createSignal} from "solid-js";

export interface BundleCoordinates {
  bundleName: string,
  bundleVersion: string,
}

export const [bundleCoordinates, setBundleCoordinates] = createSignal({bundleName: "foo", bundleVersion: "bar"});

const fetchBundle = async ({bundleName, bundleVersion}: BundleCoordinates/*, bundleVersion: string*/) => {
  const response = await fetch(`/api/bundles/${bundleName}/${bundleVersion}`)
  let json = await response.json();
  console.log(json);
  return json;
}


export const bundle = createRoot(() => {
  return createResource(bundleCoordinates, fetchBundle)[0];
});


