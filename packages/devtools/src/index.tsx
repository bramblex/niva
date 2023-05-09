import 'normalize.css/normalize.css'
import './common.scss';
import './i18n/index';

import React from 'react';
import ReactDOM from 'react-dom/client';
import { App } from './app';

window.addEventListener("contextmenu", (e) => {
  e.preventDefault();
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

root.render(<App />);