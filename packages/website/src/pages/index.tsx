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
    title: '系统 WebView',
    description: <>Niva 使用系统 WebView，不把 Chromium 内核或 Node.js 运行时打进应用。完整产物大小因平台、应用资源和构建选项而异。</>,
    image: 'feature-lightweight.webp',
    alt: '蓝色羽毛托起轻薄的桌面应用窗口',
  },
  {
    title: '极易用',
    description: <>使用<strong>前端技术</strong>和项目配置即可构建桌面应用。Windows 可输出单个 EXE，macOS 输出标准 <code>.app</code> 应用包；运行时要求按目标平台和产物说明确认。</>,
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
    description: <>提供 <strong>Windows</strong> 与 <strong>macOS</strong> 构建目标，并使用统一项目配置。请在对应设备验证产物；逐平台验收状态见<a href="https://github.com/bramblex/niva/blob/main/docs/release-0.10.0-beta.1.md">候选记录</a>。</>,
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
