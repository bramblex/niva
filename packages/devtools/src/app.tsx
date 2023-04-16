import classNames from 'classnames';
import { PropsWithChildren, useEffect, useState } from 'react';
// import { DocTab } from './doc';
import { ProjectTab } from './project';
import './app.scss';

/** 窗口控制操作区 */
export function WindowControl() {
  return     <div className="title-bar-controls">
    <button aria-label="Close" onClick={() => Niva.api.process.exit()}></button>
    <button aria-label="Minimize" onClick={() => {console.log('Minimize click'); /** Niva.api.window.setMinimized(true) */}}></button>
    <button aria-label="Fullscreen" onClick={() => Niva.api.process.exit()}></button>
  </div>
}

/** Titlebar */
export function Titlebar() {
  return <div className="title-bar" onMouseDownCapture={(ev) => {
    const t = ev.target as HTMLElement;
    if (t.tagName !== 'BUTTON' && t.tagName !== 'A') {
      Niva.api.window.dragWindow();
    }
  }}>
    <WindowControl></WindowControl>
    <div className="title-bar-text"><img className="window-icon" src="logo.png" alt="" />NIVA DEVTOOLS</div>
  </div>
}

function WindowFrame(props: PropsWithChildren<{}>) {
  const [active, setActive] = useState(true);
  const [systemInfo, setSystemInfo] = useState({
    os: '',
    arch: '',
    version: '',
  });
  const [version, setVersion] = useState('')
  const platform = systemInfo.os.toLowerCase().split(' ')[0]

  useEffect(() => {
    const handler = (_: string, { focused }: { focused: boolean }) => setActive(focused);
    Niva.addEventListener('window.focused', handler);
    Niva.api.os.info().then(setSystemInfo);
    Niva.api.process.version().then(setVersion);
    return () => {
      Niva.removeEventListener('window.focused', handler);
    };
  }, []);

  return (<div className={classNames("window", { active }, `os-${platform}`)}>
    <Titlebar></Titlebar>
    <div className="window-body has-space">
      {props.children}
    </div>
    <div className="status-bar">
      <span className="status-bar-field" onClick={() => {
          Niva.api.process.open('https://github.com/bramblex/niva');
        }}>
        <i className="icon-sm icon-config"></i>设置
      </span>
      <span className="status-bar-field" onClick={() => {
          Niva.api.process.open('https://github.com/bramblex/niva');
        }}>
        <i className="icon-sm icon-coffee"></i>请作者喝咖啡
      </span>
      <p className="status-bar-field flex-end">
        System: {systemInfo.os} {version}
      </p>
    </div>
  </div>)
}

export function App() {
  // const [tab, setTab] = useState(0);

  return <WindowFrame>
    {/* <section className="tabs">
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
    </section> */}
    <ProjectTab />
  </WindowFrame>
}
