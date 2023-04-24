import 'normalize.css/normalize.css'
// import '7.css/dist/7.css';
import './style.css';
import './i18n/index';

import React from 'react';
import ReactDOM from 'react-dom/client';
import { App } from './app';
import { Modal } from './modal';


Niva.addEventListener('*', (event, data) => {
  console.log(event, data);
});

const root = ReactDOM.createRoot(
  document.getElementById('root') as HTMLElement
);

root.render(<>
    <App />
    <Modal />
  </>);