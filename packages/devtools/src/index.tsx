import "normalize.css/normalize.css";
import "./common.scss";
import "./i18n/index";

import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./app";
import { envReady } from "./common/utils";
import { applyThemePreference, readThemePreference } from "./common/theme";
import { niva } from "./common/niva";

applyThemePreference(readThemePreference());

window.addEventListener("contextmenu", (e) => {
  // let node: HTMLElement | null = e.target as HTMLElement;
  // while (node) {
  //   node = node.parentElement;
  // }
  e.preventDefault();
});

window.addEventListener("keydown", (e) => {
  if (e.key === "r" && e.ctrlKey) {
    e.preventDefault();
  }
});

Niva.addEventListener("*", (event, data) => {
  console.log(`[Event] ${event}`, data);
});

const _call = niva.bridge.call.bind(niva.bridge);
niva.bridge.call = function (method, args) {
  console.log(`[Call] ${method}`, args);
  return _call(method, args);
};

niva.window.blockCloseRequested(true);

const root = ReactDOM.createRoot(
  document.getElementById("root") as HTMLElement
);

envReady(() => root.render(<App />));
