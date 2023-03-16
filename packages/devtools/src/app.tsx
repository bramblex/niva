import classNames from 'classnames';
import { PropsWithChildren, useEffect, useState } from 'react';
import { DocTab } from './doc';
import { ProjectTab } from './project';


function WindowFrame(props: PropsWithChildren<{}>) {
  const [active, setActive] = useState(true);
  const [systemInfo, setSystemInfo] = useState({
    os: '',
    arch: '',
    version: '',
  });

  useEffect(() => {
    const handler = (_: string, { focused }: { focused: boolean }) => setActive(focused);
    TauriLite.addEventListener('window.focused', handler);
    TauriLite.api.os.info().then(setSystemInfo);
    return () => {
      TauriLite.removeEventListener('window.focused', handler);
    };
  }, []);

  return (<div className={classNames("window", { active })}>
    <div className="title-bar" onMouseDownCapture={() => TauriLite.api.window.dragWindow()}>
      <div className="title-bar-text">Tauri Lite Devtools</div>
      <div className="title-bar-controls">
        <button aria-label="Minimize" onClick={() => TauriLite.api.window.setMinimized(true)}></button>
        <button aria-label="Close" onClick={() => TauriLite.api.process.exit()}></button>
      </div>
    </div>
    <div className="window-body has-space">
      {props.children}
    </div>
    <div className="status-bar">
      <p className="status-bar-field">
        Doc & Issue: <span className='link' onClick={() => {
          TauriLite.api.process.open('https://github.com/bramblex/tauri_lite');
        }}>https://github.com/bramblex/tauri_lite</span>
      </p>
      <p className="status-bar-field" style={{ flex: '0 0 auto' }}>
        System: {systemInfo.os} {systemInfo.arch} {systemInfo.version}
      </p>
    </div>
  </div>)
}

export function App() {
  const [tab, setTab] = useState(0);

  return <WindowFrame>
    <section className="tabs">
      <menu className="tabs-menu" role="tablist" aria-label="App Tabs">
        <button role="tab" aria-controls="project-tab" aria-selected={tab === 0} onClick={() => setTab(0)}>项目构建</button>
        <button role="tab" aria-controls="doc-tab" aria-selected={tab === 1} onClick={() => setTab(1)}>文档示例</button>
      </menu>
      <article className='tabs-panel' role="tabpanel" id="project-tab" hidden={tab !== 0}>
        <ProjectTab />
      </article>
      <article className='tabs-panel' role="tabpanel" id="doc-tab" hidden={tab !== 1}>
        <DocTab />
      </article>
    </section>
  </WindowFrame>
}
