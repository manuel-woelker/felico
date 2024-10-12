import styles from "./App.module.css"

export const App = (props) => (
    <>
      <div class={styles.app}>
        <div class={styles.navbar}>😺 felico</div>
        {props.children}
      </div>
    </>
);
