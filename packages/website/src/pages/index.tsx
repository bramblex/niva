import React from 'react';
import Link from '@docusaurus/Link';
import useBaseUrl from '@docusaurus/useBaseUrl';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Layout from '@theme/Layout';
import styles from './index.module.css';

function Arrow() {
  return <svg aria-hidden="true" viewBox="0 0 20 20" fill="none"><path d="M4 10h11m-4-4 4 4-4 4" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" /></svg>;
}

const capabilities = [
  {
    title: '超轻量',
    description: <>构建的桌面应用最小只有 <strong>3MB</strong>，仅有 Electron 的 <strong>1/10</strong>。Niva 仅依赖系统原生的 Webview，不依赖 Chromium 或者 Node.js，极致的轻量。</>,
    image: 'feature-lightweight.webp',
    alt: '蓝色羽毛托起轻薄的桌面应用窗口',
  },
  {
    title: '极易用',
    description: <>仅使用<strong>前端技术</strong>，不需要学习复杂的 Node.js 和 Electron API 也不需要复杂的配置，即可构建出一个桌面应用。构建<strong>单可执行文件</strong>，无需安装，点击即用。</>,
    image: 'feature-easy.webp',
    alt: '点击网页界面后得到可直接打开的桌面应用',
  },
  {
    title: '图形化',
    description: <>Niva 提供图形化界面的开发工具，<strong>一键点击构建</strong>桌面应用，无需复杂的命令行操作，也无需安装 Node 环境。</>,
    image: 'feature-devtools.webp',
    alt: '画笔在带有配置控件的图形编辑器中调整应用界面',
  },
  {
    title: '跨平台',
    description: <>同时支持 <strong>Windows</strong>、<strong>macOS</strong>，无需额外的配置，即可构建出跨平台的桌面应用。</>,
    image: 'feature-platforms.webp',
    alt: '同一个应用出现在两种不同的桌面电脑上',
  },
];

export default function Home(): JSX.Element {
  const { siteConfig } = useDocusaurusContext();
  const logo = useBaseUrl('img/logo.png');
  const screenshot = useBaseUrl('img/devtools-screenshot.webp');
  const imageBase = useBaseUrl('img/');
  return (
    <Layout title={`${siteConfig.title} - ${siteConfig.tagline}`} description={siteConfig.tagline}>
      <main>
        <section className={styles.hero}>
          <div className={styles.heroInner}>
            <div className={styles.heroCopy}>
              <div className={styles.heroBrand}><img src={logo} alt="" /></div>
              <h1>{siteConfig.title}</h1>
              <p>{siteConfig.tagline}</p>
              <div className={styles.heroActions}>
                <Link className={styles.primaryAction} to="/docs/intro">快速上手 <Arrow /></Link>
                <Link className={styles.secondaryAction} to="https://github.com/bramblex/niva/releases">下载 <Arrow /></Link>
              </div>
            </div>
            <img className={styles.previewScreenshot} src={screenshot} alt="Niva Devtools 的真实项目总览界面" width={968} height={668} />
          </div>
        </section>
        <section className={styles.capabilities} aria-label="Niva 能做什么">
          <div className={styles.capabilitiesInner}>
            <div className={styles.featureGrid}>
              {capabilities.map((item) => (
                <article className={styles.capability} key={item.title}>
                  <div className={styles.featureCopy}><h3>{item.title}</h3><p>{item.description}</p></div>
                  <img src={imageBase + item.image} alt={item.alt} width={1200} height={900} loading="lazy" />
                </article>
              ))}
            </div>
          </div>
        </section>
      </main>
    </Layout>
  );
}
