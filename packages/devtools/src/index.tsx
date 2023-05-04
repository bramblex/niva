import 'normalize.css/normalize.css'
import './common.scss';
import './i18n/index';

import React from 'react';
import ReactDOM from 'react-dom/client';
import { App } from './app';


Niva.addEventListener('*', (event, data) => {
  console.log(`[Event] ${event}`, data);
});

const _call = Niva.call;
Niva.call = function (method, args) {
  console.log(`[Call] ${method}`, args);
  return _call(method, args);
}

const root = ReactDOM.createRoot(
  document.getElementById('root') as HTMLElement
);

root.render(<App />);