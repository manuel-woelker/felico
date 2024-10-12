import {useParams} from "@solidjs/router";
import styles from "./BundleView.module.css"
import {bundle, setBundleCoordinates} from "../stores/BundleStore";
import {createEffect} from "solid-js";

export const BundleView = (props) => {
  const params = useParams();
  createEffect(() => {
    setBundleCoordinates({bundleName: params.bundleName, bundleVersion: params.bundleVersion});
  })
  console.log(JSON.stringify(params));
  return (
      <>
        <div class={styles.main}>
          <div class={styles.sidebar}>
            <div class={styles.bundleName}>{params.bundleName}</div>
            {params.bundleVersion}
            <div>
            </div>
            <div>
              x{JSON.stringify(bundle())}x
            </div>
          </div>
          <div>body</div>
        </div>
      </>
  );
}
