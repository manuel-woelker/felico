import {Component} from 'solid-js';

import {BundleIndex} from "../stores/BundleIndexStore";
import {A} from "@solidjs/router";
import styles from "./Home.module.css";


export const Home: Component = () => {


  interface ErrorMessage {
    error: string
  }

  (async function getBundleIndex() {
    try {
      const error_response = await fetch('/api/test_error')
      const error_json = (await error_response.json()) as ErrorMessage
      console.log(error_json)
      console.log(error_json.error)
      const response = await fetch('/api/bundles')
      const bundleIndex = (await response.json()) as BundleIndex
      console.log(bundleIndex)
      for (const bundleInfo of bundleIndex.bundles) {
        const response = await fetch(
            `/api/bundles/${bundleInfo.name}/${bundleInfo.version}`,
        )
        const bundleIndex = (await response.json()) as BundleIndex
        console.log(bundleIndex)
      }
    } catch (error) {
      console.error(error)
    }
  })();
  return (
      <div class={styles.home}>
        <h3>Welcome to felico</h3><br/>
        Read the <A href="/docs">Bundle docs</A>
      </div>
  );
};

