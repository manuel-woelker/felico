/* @refresh reload */
import {render} from 'solid-js/web';
import {Route, Router} from "@solidjs/router";

import './index.css';
import {App} from './layouts/App';
import {Home} from "./views/Home";
import {BundleView} from "./docs/BundleView";
import {BundleDocsOverview} from "./docs/BundleDocsOverview";

const root = document.getElementById('root');

if (import.meta.env.DEV && !(root instanceof HTMLElement)) {
  throw new Error(
      'Root element not found. Did you forget to add it to your index.html? Or maybe the id attribute got misspelled?',
  );
}

//render(() => <App />, root!);
render(() => <Router root={App}>
  <Route path="/" component={Home}/>
  <Route path="/docs" component={BundleDocsOverview}/>
  <Route path="/docs/:bundleName/:bundleVersion" component={BundleView}/>
</Router>, root!);
