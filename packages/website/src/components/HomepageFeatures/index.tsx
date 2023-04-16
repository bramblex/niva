import React from 'react';
import clsx from 'clsx';
import styles from './styles.module.css';

type FeatureItem = {
  title: string;
  Icon: JSX.Element;
  description: JSX.Element;
};

function Red(props: any) {
  return <span style={{ color: '#FF6347', fontWeight: 'bold' }}>{props.children}</span>
}

const FeatureList: FeatureItem[] = [
  {
    title: '超轻量',
    Icon: <img
      className={styles.featureImage}
      src={require('@site/static/img/lightweight.png').default} />,
    description: (
      <>
        构建的桌面应用最小只有 <Red>3MB</Red>，仅有 Electron 的 <Red>1/10</Red>。Niva 仅依赖系统原生的 Webview，不依赖 Chromium 或者 Node.js，极致的轻量。
      </>
    ),
  },
  {
    title: '极易用',
    Icon: <img
      className={styles.featureImage}
      src={require('@site/static/img/easy-to-use.png').default} />,
    description: (
      <>
        仅使用<Red>前端技术</Red>，不需要学习复杂的 Node.js 和 Electron API 也不需要复杂的配置，即可构建出一个桌面应用。构建<Red>单可执行文件</Red>，无需安装，点击即用。
      </>
    ),
  },
  {
    title: '图形化',
    Icon: <img
      className={styles.featureImage}
      src={require('@site/static/img/illustration.png').default} />,
    description: (
      <>
        Niva 提供图形化界面的开发工具，<Red>一键点击构建</Red>桌面应用，无需复杂的命令行操作，也无需安装 Node 环境。
      </>
    ),
  },
  {
    title: '跨平台',
    Icon: <img
      className={styles.featureImage}
      src={require('@site/static/img/pc.png').default} />,
    description: (
      <>
        同时支持 <Red>Windows</Red>、<Red>macOS</Red>，无需额外的配置，即可构建出跨平台的桌面应用。
      </>
    ),
  },
];

function Feature({ title, Icon, description }: FeatureItem) {
  return (
    <div className={clsx('col col--3')}>
      <div className="text--center">
        {Icon}
      </div>
      <div className="text--center padding-horiz--md">
        <h3>{title}</h3>
        <p>{description}</p>
      </div>
    </div>
  );
}

export default function HomepageFeatures(): JSX.Element {
  return (
    <section className={styles.features}>
      <div className="container">
        <div className="row">
          {FeatureList.map((props, idx) => (
            <Feature key={idx} {...props} />
          ))}
        </div>
      </div>
    </section>
  );
}
