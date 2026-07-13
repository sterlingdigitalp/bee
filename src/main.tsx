import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import Widget from "./Widget";
import "./styles.css";

const params = new URLSearchParams(location.search);
const isWidget = params.get("window") === "widget" || location.hash === "#widget";
createRoot(document.getElementById("root")!).render(<StrictMode>{isWidget ? <Widget /> : <App />}</StrictMode>);
