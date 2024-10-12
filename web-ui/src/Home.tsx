import {Component, For} from 'solid-js';

import styles from './App.module.css';
import {PackageIndex, packageIndexStore} from "./stores/PackageIndexStore";
import {A} from "@solidjs/router";

export const Home: Component = () => {


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
  })();
  return (
      <div class={styles.App}>
        <header class={styles.header}>
          <For each={packageIndexStore.packages}>
            {(pkg, index) =>
                <div><A href={`/docs/${pkg.name}/${pkg.version}`}>{pkg.name} {pkg.version}</A></div>
            }
          </For>
        </header>
      </div>
  );
};

