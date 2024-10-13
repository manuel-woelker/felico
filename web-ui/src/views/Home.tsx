import {Component} from 'solid-js';

import {PackageIndex} from "../stores/PackageIndexStore";
import {A} from "@solidjs/router";
import styles from "./Home.module.css";


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
      for (const packageInfo of packageIndex.packages) {
        const response = await fetch(
            `/api/packages/${packageInfo.name}/${packageInfo.version}`,
        )
        const packageDescription = (await response.json()) as PackageIndex
        console.log(packageDescription)
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

