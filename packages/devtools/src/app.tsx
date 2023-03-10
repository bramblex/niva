import { useLocalModel, useModel, useModelContext, useModelProvider } from '@bramblex/state-model-react';
import { PropsWithChildren } from 'react';
import { ImportPage } from './import-page';
import { ProjectModel } from './project-model';
import { ProjectPage } from './project-page';

export function App() {
  const project = useLocalModel(() => new ProjectModel());
  const Provider = useModelProvider(ProjectModel);

  return <Provider value={project}>
    <AppInner />
  </Provider>
}

function AppInner() {
  const project = useModelContext(ProjectModel);
  const { state } = useModel(project);
  return state ? <ProjectPage /> : <ImportPage />;
}

export function Page(prop: PropsWithChildren<{ title: string }>) {
  const { title, children } = prop;
  return <div className='app-page-container'>
    <div className='app-page-title'>{title}</div>
    <div className='app-page-content'>
      {children}
    </div>
    <div className='app-page-footer'>
      Doc & Issue: <span className='link' onClick={() => {
        TauriLite.api.process.open({ uri: 'https://github.com/bramblex/tauri-lite' });
      }}>https://github.com/bramblex/tauri-lite</span>
    </div>
  </div>
}
