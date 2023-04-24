import classNames from 'classnames';
import { PropsWithChildren, useEffect, useState } from 'react';
// import { DocTab } from './doc';
import { ProjectTab } from './project';
import './app.scss';
import { useTranslation } from 'react-i18next'

/** 窗口控制操作区 */
export function WindowControl(props : { os: string}) {
  const { os } = props
  const [isMaximized, setMaximized] = useState(false)

  let closeIcon, minimizeIcon, maximizeIcon = null, restoreMaximizeIcon = null
  if (os === 'mac') { // macOS
    closeIcon = <svg x="0px" y="0px" width="10px" height="10px" viewBox="0 0 20 20"><polygon fill="#4d0000" points="15.9,5.2 14.8,4.1 10,8.9 5.2,4.1 4.1,5.2 8.9,10 4.1,14.8 5.2,15.9 10,11.1 14.8,15.9 15.9,14.8 11.1,10 "/></svg>
    minimizeIcon = <svg x="0px" y="0px" width="10px" height="10px" viewBox="0 0 20 20"><rect fill="#995700" x="2.4" y="9" width="15.1" height="2"/></svg>
    maximizeIcon = <svg x="0px" y="0px" width="10px" height="10px" viewBox="0 0 20 20" data-radium="true"><path fill="#006400" d="M5.3,16H13L4,7v7.7C4.6,14.7,5.3,15.4,5.3,16z"></path><path fill="#006400" d="M14.7,4H7l9,9V5.3C15.4,5.3,14.7,4.6,14.7,4z"></path></svg>
    restoreMaximizeIcon = <svg x="0px" y="0px" width="10px" height="10px" viewBox="0 0 10 10"><path fill="#006400" d="M5,10c0,0 0,-2.744 0,-4.167c0,-0.221 -0.088,-0.433 -0.244,-0.589c-0.156,-0.156 -0.368,-0.244 -0.589,-0.244c-1.423,0 -4.167,0 -4.167,0l5,5Z" /><path fill="#006400" d="M5,0c0,0 0,2.744 0,4.167c0,0.221 0.088,0.433 0.244,0.589c0.156,0.156 0.368,0.244 0.589,0.244c1.423,0 4.167,0 4.167,0l-5,-5Z" /></svg>
  } else { // windows
    minimizeIcon = <svg x="0px" y="0px" viewBox="0 0 10.2 1" width="10px" height="10px"><rect fill="rgba(0, 0, 0, .4)" width="10.2" height="1"/></svg>
    maximizeIcon = <svg x="0px" y="0px" viewBox="0 0 10.2 10.1" width="10px" height="10px"><path fill="rgba(0, 0, 0, .4)" d="M0,0v10.1h10.2V0H0z M9.2,9.2H1.1V1h8.1V9.2z"/></svg>
    restoreMaximizeIcon = <svg x="0px" y="0px" viewBox="0 0 10.2 10.2" width="10px" height="10px"><path fill="rgba(0, 0, 0, .4)" d="M2.1,0v2H0v8.1h8.2v-2h2V0H2.1z M7.2,9.2H1.1V3h6.1V9.2z M9.2,7.1h-1V2H3.1V1h6.1V7.1z"/></svg>
    closeIcon = <svg x="0px" y="0px" viewBox="0 0 10.2 10.2" width="10px" height="10px"><polygon fill="rgba(0, 0, 0, .4)" points="10.2,0.7 9.5,0 5.1,4.4 0.7,0 0,0.7 4.4,5.1 0,9.5 0.7,10.2 5.1,5.8 9.5,10.2 10.2,9.5 5.8,5.1 "/></svg>
  }
  
  return     <div className="title-bar-controls">
    <button aria-label="Minimize" onClick={() => Niva.api.window.setMinimized(true) }>{ minimizeIcon }</button>
    <button aria-label="Fullscreen" onClick={() => { 
      setMaximized(!isMaximized)
      Niva.api.window.setMaximized()
    }}>
      {isMaximized ? restoreMaximizeIcon : maximizeIcon}
    </button>
    <button aria-label="Close" onClick={() => Niva.api.process.exit()}>{ closeIcon }</button>
  </div>
}

/** Titlebar */
export function Titlebar(props : { os: string}) {
  const { os } = props
  return <div className="title-bar" onMouseDownCapture={(ev) => {
    const t = ev.target as HTMLElement;
    if (t.tagName !== 'BUTTON' && t.tagName !== 'A') {
      Niva.api.window.dragWindow();
    }
  }}>
    <WindowControl os={os}></WindowControl>
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
    Niva.api.window.setResizable(true);
    return () => {
      Niva.removeEventListener('window.focused', handler);
    };
  }, []);

  const { t } = useTranslation()

  return (<div className={classNames("window", { active }, `os-${platform}`)}>
    <Titlebar os={platform}></Titlebar>
    <div className="window-body has-space">
      {props.children}
    </div>
    <div className="status-bar">
      <span className="status-bar-field" onClick={() => {
          Niva.api.process.open('https://bramblex.github.io/niva/en/docs/intro');
        }}>
        <i className="icon-sm icon-config"></i>{t('setting')}
      </span>
      <span className="status-bar-field" onClick={() => {
          Niva.api.process.open('https://github.com/bramblex/niva');
        }}>
        <i className="icon-sm icon-coffee"></i>{t('buycoffee')}
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
