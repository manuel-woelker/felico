import {A} from "@solidjs/router";
import {For} from "solid-js";
import {bundleIndexStore} from "../stores/BundleIndexStore";
import styles from "./BundleDocsOverview.module.css";


export const BundleDocsOverview = (props: {}) => {
  return (
      <div class={styles.main}>
        <h4>Bundles</h4>
        <For each={bundleIndexStore.bundles}>{(bundle) =>
            <div><A href={`/docs/${bundle.name}/${bundle.version}`}>{bundle.name} {bundle.version}</A></div>
        }</For>
      </div>
  );
}
