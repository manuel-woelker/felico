import {useParams} from "@solidjs/router";

export const BundleView = (props) => {
  const params = useParams();
  console.log(JSON.stringify(params));
  return (
      <>
        <h2>Bundle view {params.bundleName} {params.bundleVersion}</h2>
      </>
  );
}
