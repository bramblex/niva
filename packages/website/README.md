# Niva 文档站

文档站使用 Docusaurus 3。需要 Node.js 20 或更新版本；依赖由本目录的
`package-lock.json` 固定。

```bash
cd packages/website
npm ci
npm run start
```

发布前构建并检查站内链接：

```bash
npm run build
npm run serve
```

站点入口、导航和主题配置在 `docusaurus.config.js`；首页在 `src/pages/`；
API 页面在 `docs/api/`。首页 logo 和配色与当前 Devtools 共用视觉来源。
