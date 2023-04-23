import classNames from 'classnames';
import { PropsWithChildren, useEffect, useState } from 'react';
// import { DocTab } from './doc';
import { ProjectTab } from './project';
import './app.scss';

/** 窗口控制操作区 */
export function WindowControl() {
  const [isMaximized, setMaximized] = useState(false)

  const closeIcon = <svg x="0px" y="0px" width="10px" height="10px" viewBox="0 0 20 20"><polygon fill="#4d0000" points="15.9,5.2 14.8,4.1 10,8.9 5.2,4.1 4.1,5.2 8.9,10 4.1,14.8 5.2,15.9 10,11.1 14.8,15.9 15.9,14.8 11.1,10 "/></svg>
  const minimizeIcon = <svg x="0px" y="0px" width="10px" height="10px" viewBox="0 0 20 20"><rect fill="#995700" x="2.4" y="9" width="15.1" height="2"/></svg>
  const maximizeIcon = <svg x="0px" y="0px" width="10px" height="10px" viewBox="0 0 20 20" data-radium="true"><path fill="#006400" d="M5.3,16H13L4,7v7.7C4.6,14.7,5.3,15.4,5.3,16z"></path><path fill="#006400" d="M14.7,4H7l9,9V5.3C15.4,5.3,14.7,4.6,14.7,4z"></path></svg>
  const restoreMaximizeIcon = <svg x="0px" y="0px" width="10px" height="10px" viewBox="0 0 10 10"><path fill="#006400" d="M5,10c0,0 0,-2.744 0,-4.167c0,-0.221 -0.088,-0.433 -0.244,-0.589c-0.156,-0.156 -0.368,-0.244 -0.589,-0.244c-1.423,0 -4.167,0 -4.167,0l5,5Z" /><path fill="#006400" d="M5,0c0,0 0,2.744 0,4.167c0,0.221 0.088,0.433 0.244,0.589c0.156,0.156 0.368,0.244 0.589,0.244c1.423,0 4.167,0 4.167,0l-5,-5Z" /></svg>
  
  return     <div className="title-bar-controls">
    <button aria-label="Close" onClick={() => Niva.api.process.exit()}>{ closeIcon }</button>
    <button aria-label="Minimize" onClick={() => Niva.api.window.setMinimized(true) }>{ minimizeIcon }</button>
    <button aria-label="Fullscreen" onClick={() => { 
      setMaximized(!isMaximized)
      // Niva.api.window.setMaximized(!isMaximized)
    }}>
      {isMaximized ? restoreMaximizeIcon : maximizeIcon}
    </button>
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
    <ProjectTab />
  </WindowFrame>
}
