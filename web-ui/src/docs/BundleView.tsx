import {useParams} from "@solidjs/router";
import styles from "./BundleView.module.css"
import {bundle, setBundleCoordinates} from "../stores/BundleStore";
import {createEffect, For, Show} from "solid-js";

export const BundleView = (props: {}) => {
  const params = useParams();
  createEffect(() => {
    setBundleCoordinates({bundleName: params.bundleName, bundleVersion: params.bundleVersion});
  })
  console.log(JSON.stringify(params));
  return (
      <>
        <div class={styles.main}>
          <div class={styles.sidebar}>
            Bundle
            <div class={styles.bundleName}>{params.bundleName}</div>
            {params.bundleVersion}
            <div>
            </div>
          </div>
          <div class={styles.center}>
            <div>
              <Show when={bundle() && bundle().functions}>
                <div class={styles.bundleName}>Bundle {params.bundleName}</div>
                <h4>Functions</h4>
                <For each={bundle().functions}>{(fn => <div>
                  {fn.name} {fn.signature}
                </div>)}</For>
              </Show>
            </div>
          </div>
        </div>
      </>
  );
}
