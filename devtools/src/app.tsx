import './app.css';
import { ImportProject } from './import-project';
import { DialogRoot } from './dialog';
import { devtools } from './model/app';

devtools.init();

export function App() {
  return <>
    <ImportProject />
    <DialogRoot />
  </>;
}