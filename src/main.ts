import "./styles.css";
import "./app.css"
import App from "./App.svelte";

const app = new App({
  // The app element is present in index.html, so assertion is safe
  target: document.getElementById("app")!,
});

export default app;
