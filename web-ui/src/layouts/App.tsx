import styles from "./App.module.css"
import {A} from "@solidjs/router";

export const App = (props: {}) => (
    <>
      <div class={styles.app}>
        <div class={styles.navbar}><A href="/">😺 felico</A> <A href="/docs">Bundle docs</A></div>
        {props.children}
      </div>
    </>
);
