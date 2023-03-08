import './style.css';

import React from 'react';
import ReactDOM from 'react-dom/client';
import { App } from './app';

TauriLite.addEventListener('*', (event, data) => {
  console.log(event, data);
});

const root = ReactDOM.createRoot(
  document.getElementById('root') as HTMLElement
);

root.render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);