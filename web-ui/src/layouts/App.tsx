import styles from "./App.module.css"
import {A} from "@solidjs/router";
import {ParentProps} from "solid-js";

export const App = (props: ParentProps) => (
    <>
      <div class={styles.app}>
        <div class={styles.navbar}>😺 <A href="/">felico</A><A href="/docs">Bundle docs</A></div>
        {props.children}
      </div>
    </>
);
