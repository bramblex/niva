import "normalize.css/normalize.css";
import "./common.scss";
import "./i18n/index";
import "types";

import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./app";
import { envReady } from "./common/utils";

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

const _call = Niva.call;
Niva.call = function (method, args) {
  console.log(`[Call] ${method}`, args);
  return _call(method, args);
};

Niva.api.window.blockCloseRequested(true);

const root = ReactDOM.createRoot(
  document.getElementById("root") as HTMLElement
);

envReady(() => root.render(<App />));
