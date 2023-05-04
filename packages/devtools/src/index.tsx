import 'normalize.css/normalize.css'
import './common.scss';
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