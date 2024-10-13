import {A} from "@solidjs/router";
import {For} from "solid-js";
import {packageIndexStore} from "../stores/PackageIndexStore";

export const BundleDocsOverview = (props) => {
  return (
      <div>
        <h4>Bundles</h4>
        <For each={packageIndexStore.packages}>{(bundle) =>
            <div><A href={`/docs/${bundle.name}/${bundle.version}`}>{bundle.name} {bundle.version}</A></div>
        }</For>
      </div>
  );
}
