import 'normalize.css'
import '7.css/dist/7.css';
import './style.css';

import React from 'react';
import ReactDOM from 'react-dom/client';
import { App } from './app';
import { Modal } from './modal';

TauriLite.addEventListener('*', (event, data) => {
  console.log(event, data);
});

const root = ReactDOM.createRoot(
  document.getElementById('root') as HTMLElement
);

root.render(
  <React.StrictMode>
    <App />
    <Modal />
  </React.StrictMode>
);