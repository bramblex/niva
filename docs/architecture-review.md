# Niva 整体架构 Review 档案

建立日期：2026-09-24。当前阶段：按2026-09-25用户最新指令进入实现；IPC fallback具体接口已授权执行方决定。实施范围、阶段与验证见architecture-implementation-plan.md。

当前有效方案见文末「R15 当前方案与待决项」；此前MSI/WiX等已被替代的讨论保留作历史，不是实现指令。

## 1. 目标与工作约定

> 以下为初始review阶段约定。用户后续已明确要求规划并落实全部有效决定；实现阶段授权更新见文末与architecture-implementation-plan.md，旧的只读阶段限制不再适用于本次已授权实现。

目标是让项目负责人能解释整个项目的大致设计：有哪些组件，各自负责什么，如何协作，关键选择的理由与代价，平台差异，以及哪些能力已经实现、哪些仍缺验证。不是逐行读代码，也不是只列 bug。

- 本阶段只允许更新本 review 档案。源码、配置、依赖、已有文档、Git 暂存区和合并状态均不修改。
- 发现问题只批注，不顺手修复、不生成实现补丁、不安装依赖、不启动服务或运行会产生副作用的验证。
- 所有章节完成讲解、问题批注和决策汇总后，再由用户明确开启实现阶段。章节确认或接受建议不等于授权实现。
- 每轮只推进一个章节中的一个完整主题。较大的章节拆成多轮，不一次性倾倒全部细节。
- 阅读依据优先使用当前源码。设计文档说明意图，测试源码说明验证设计，历史结果仅说明当时运行情况。
- 不把“源码存在”“测试已写”“历史报告通过”“当前运行通过”和“目标平台验收”混为一谈。
- 历史记忆仅用于提醒检查点，不能代替本轮证据。当前工作区发生外部变化时记录新基线和受影响章节，不代为合并或修复。

## 2. 初始基线

| 项目 | 2026-09-24 观察 |
|---|---|
| 仓库 | `/Users/brambles/Workspace/playground/niva` |
| 当前分支 | `codex/merge-cross-platform-packager` |
| HEAD | `7e1e316f802a0682fc48ea43517227d7663450e9` |
| 工作区 | 有已暂存改动；`Cargo.lock` 为 `UU`，合并冲突未解决 |
| 当前 review 对象 | 当前工作树中的源码，包含未提交打包改动；不能只用 HEAD 代表这些内容 |
| 主要变更区域 | `crates/niva_packager/`、Devtools 多目标构建界面、CI/打包脚本及文档 |
| 本轮验证 | Git 状态、目录、清单、入口源码与文档的只读盘点；没有构建、测试或真机运行 |
| 基线限制 | 可继续讨论模块和设计；依赖解析、完整构建与合并完成状态不作确认 |

后续每章记录实际查阅版本、关键符号及文件位置。工作树文件以本档末尾的 SHA-256 清单辅助识别，不把移动中的行号作为唯一依据。

## 3. Review 路线与覆盖表

各章均为“待讲解”。规划完成不代表以下内容已完成 review。

| 编号 | 主题 | 要掌握的设计细节 / 本章交付 | 主要证据入口 |
|---|---|---|---|
| R01 | 产品边界与整体分层 | 应用开发者、最终用户、框架开发者分别使用什么；Rust、系统 WebView、业务前端、Devtools、NodeCompat、打包器的职责；开发期和运行期的区别；形成组件与依赖清单 | 根 `Cargo.toml`、`package.json`、各包清单、`crates/niva/src/main.rs` |
| R02 | 启动、配置与退出 | 参数、`niva.json`、平台覆盖、默认值如何合并；debug 与 packaged 两条启动路径；窗口 0、ready、退出和清理的时序 | `app/mod.rs`、`app/options.rs`、`app/event_handler.rs` |
| R03 | 资源、页面与存储 | 目录资源与包内资源、索引与压缩、路径归一化、MIME；自定义协议与本地 HTTP 的边界；origin、导航与浏览器存储；列一条页面加载链路 | `app/resource_manager/`、`app/assets.rs`、`app/custom_protocol.rs`、`app/http_server/` |
| R04 | Bridge 与协议 | 注入脚本、WS/平台 IPC 的选择、请求和响应、事件、二进制和流、错误码、取消、断连、iframe 路由；走读一次调用和一次流式任务 | `assets/initialize_script.js`、`app/api_manager/`、`app/ipc_macos.rs`、`app/ipc_windows_frames.rs`、`docs/bridge.md` |
| R05 | 权限与信任边界 | 窗口、frame、origin、token 的绑定；本地页/远端页/debug 的权限；Rust 鉴权点、文件 URL、路径穿越、导航后授权；形成来源×能力矩阵 | `app/window_manager/permissions.rs`、`builder.rs`、HTTP/IPC/API 调度；安全专题文档 |
| R06 | 并发、状态与资源所有权 | tao 主线程、异步执行、阻塞任务池如何协作；锁、回调、事件传递、取消与销毁；主线程约束和错误传播 | `app/main_exec.rs`、`app/api_manager/thread_pool.rs`、`app/utils.rs`、窗口和事件管理 |
| R07 | 窗口与桌面交互 | 多窗口/父子窗口/owner、WebView 生命周期、菜单、托盘、全局快捷键、对话框、剪贴板、显示器；跨平台实现与不支持项 | `app/window_manager/`、`menu/`、`tray_manager/`、`shortcut_manager/`、对应 API |
| R08 | 原生 API 与系统 I/O | 注册方式、类型与错误契约；文件、资源、进程、系统、socket/TLS 等能力如何暴露；阻塞/异步/流式取舍；权限与资源释放 | `app/api/mod.rs`、`app/api/`、`app/fs_ops.rs`、`app/os_native.rs`、`packages/types/Niva_zh.d.ts` |
| R09 | NodeCompat 装载与执行模型 | Node 风格 API 与完整 Node 运行时的区别；ESM、require、全局对象、importmap；模块选择、依赖闭包、注入条件、JS/Native 分层及同步调用 | `packages/node-compat/src/`、`runtime-files.json`、`app/node_bootstrap.rs`、`app/node_compat.rs` |
| R10 | NodeCompat 语义与覆盖 | 按纯 JS、文件/进程、网络、加密/压缩分组；回调/Promise/stream 和 WebView 限制；上游版本、筛选/排除、host fallback、分母和失败定义 | NodeCompat tests/scripts、`docs/node-api-inventory.json`、上游 conformance 与 engine-boundary 文档/结果 |
| R11 | 外部宿主与 stdio | 宿主、Niva 进程、主窗口的关系；NDJSON、ready 与页面就绪、输入上限、EOF/BrokenPipe、stdout/stderr；走读宿主往返 | `app/stdio.rs`、`app/api/host.rs`、`examples/stdio_host.py`、`docs/stdio-host-design.md` |
| R12 | Devtools 与开发工作流 | Devtools 自举、项目导入/保存、配置编辑、类型/校验、调试启动、错误呈现、历史记录、i18n；GUI 如何调用框架与构建工具 | `packages/devtools/src/`、`niva.json`、`vite.config.ts`、`packages/examples/` |
| R13 | 构建、打包与分发 | 旧构建脚本和新多目标打包器的调用关系；runtime kit、资源格式、NodeCompat 文件闭包、架构校验、图标/版本、签名、产物与运行时读回；区分宿主平台和目标平台 | `crates/niva_packager/`、`win_packager/`、`icon_creator/`、Devtools `build-scripts/`、根构建脚本、`scripts/` |
| R14 | 质量、依赖与发布边界 | Rust/TS/JS/上游/协议/GUI/真机测试各证明什么；CI 与本机记录；依赖和第三方许可证、完整产物体积、签名证据、文档网站和公开声明的一致性 | `.github/workflows/`、各 tests/examples、`packages/website/`、`packages/types/`、依赖清单、`AGENTS.md`、`docs/roadmap.md` |
| R15 | 全局回看与实现前收口 | 重新串起三条端到端链路；汇总重复/缺失职责、设计决定、未决问题、风险顺序和实现验收条件；仍不实现 | 本档全部章节、问题与决策台账 |

表内 `app/` 和 `assets/` 均相对于 `crates/niva/src/` 和 `crates/niva/`；其他路径相对于仓库根目录。

R09/R10、R12/R13 预计需要多轮。每轮讲到用户能联系上下游为止，不以固定时间或文件数量判断完成。

### 只读盘点补充（用于安排讲解，不代表已共同 review）

- 运行入口：`main.rs` → `NivaApp::new` → 窗口创建/页面注入 → JS 经 WS 或 IPC 调用 Rust；API 注册与调度分别位于 `app/api/mod.rs` 和 `app/api_manager/`。
- NodeCompat：除项目资源打包外，还需检查 `crates/niva/build.rs` 对 `runtime-files.json` 所列文件的压缩内嵌，以及 `app/node_compat.rs` 的按配置注入/提供资源。R09 和 R13 均覆盖这条链路。
- Devtools：入口是 `packages/devtools/src/index.tsx`，项目模型是 `models/project.model.ts`。既有 `ProjectModel.build()` 连接 `build-scripts/` 与 Windows 打包工具；新多目标界面和 `niva_packager` 的关系留待 R13 展开。
- 资源格式：打包端写入 `RESOURCE_INDEXES`/`RESOURCE_DATA`，运行端由 `AppResourceManager` 读取；生产者与消费者要配对 review。
- 类型与外围包：`packages/types/Niva_zh.d.ts` 为 Devtools 的本地依赖；本次盘点未见从 Rust 自动生成的路径，后续核查契约维护方式。网站为独立 Docusaurus 包，示例另列目录。`icon_creator` 仍在 Rust workspace，盘点未见当前构建路径调用，不能仅凭目录存在认定它是活动链路。

以上由 `luna_deep_worker` 提供只读证据，主线程整合到路线；尚未逐项审计 API、核对全部文档或验证平台行为。

## 4. 每轮怎么走

1. **定位**：本主题解决什么问题，在整体架构的什么位置，与上一章如何连接。
2. **正常路径**：用一个具体例子串起输入、组件、数据、线程/进程边界和输出。
3. **关键设计**：解释主要对象、状态归属、接口契约和设计取舍。代码能证明“怎么做”；没有记录的“为什么”明确标为推断。
4. **边界路径**：至少讨论一个失败、取消、关闭、越权或平台差异情形。
5. **共同讨论**：用户提出疑问、表达设计偏好；只在档案中记录问题与候选方向。
6. **回写档案**：记录已讲内容、证据、批注和剩余问题。用户确认理解后才能标记“已共同确认”。

每章完成的理解标准：能说清“它负责什么、上游怎么用它、状态/资源归谁、失败时会怎样、为什么这样分层、哪些地方还不确定”。用讨论确认，不要求背诵实现。

三条贯穿全程的案例：

- 打开一个打包应用 → 读取配置和资源 → 创建主窗口 → 页面加载 → 第一次原生 API 调用 → 关闭清理。
- 开启 NodeCompat → 选择模块并打包 → 装载模块 → 文件/网络调用 → 返回结果或取消 → 退出清理。
- 在 Devtools 导入前端项目 → 编辑配置 → 调试 → 多目标构建 → 得到分发包 → 目标机器启动。

## 5. 记录格式与状态

### 章节记录模板

```text
章节 / 本轮子主题：
状态：待讲解 / 讲解中 / 待用户确认 / 已共同确认
证据基线：HEAD + 工作树变化说明 + 日期
本轮已讲内容：
关键组件、对象和接口：
正常调用链：
状态/线程/资源所有权：
失败路径与平台差异：
设计理由：[有文档依据 / 从代码推断 / 待确认]
源码证据：文件 + 符号 + 本轮行号
验证证据：[源码 / 测试定义 / 历史结果 / 当前运行 / 平台真机]
用户疑问与讨论结论：
关联批注：AR-xxx
尚未覆盖：
用户确认：未确认 / 用户原意摘要及日期
下一步：
```

### 问题批注模板

```text
编号：AR-xxx
归属章节：
类别：缺陷 / 设计取舍 / 证据缺口 / 文档不一致 / 理解问题 / 基线问题
状态：待核实 / 已确认待决策 / 已决策待实现 / 接受现状 / 不成立
优先级：待定 / P0 阻断 / P1 高 / P2 中 / P3 低（说明依据）
观察与来源：可复核的源码、命令结果或报告；推断单独标注
触发条件与影响：
已知事实 / 尚未证实：
候选方向及代价：讨论方案，不写实现补丁
用户决定：未决定 / 决定摘要及日期
未来验收条件：
关联章节/批注：
```

“已确认待决策”只表示问题成立；“已决策待实现”不表示已经修复。证据缺口不自动认定为代码缺陷，也不自动提升为 P0。

### 设计决策台账

| 编号 | 议题 | 当前实现/约束 | 候选方向与代价 | 用户结论 | 关联批注 |
|---|---|---|---|---|---|
| D01 | 统一 API 与 Node 环境注入分离 | 当前实现的一致性仍需源码核查 | 默认通过 Niva 命名空间提供统一 API，兼容配置只控制 Node 环境注入 | 已确定方向，待实现阶段执行 | AR-002 |
| D02 | Devtools 使用统一 Node 风格 API | 当前项目模型使用 Niva.api.fs/process/os | 迁移到 fs、os、process、child_process；必要时通过 Vite 映射到 Niva 对应对象 | 已确定方向，待实现阶段执行 | AR-003 |
| D03 | 注入脚本与 NodeCompat 合并为 TypeScript 包 | 现有初始化脚本与 NodeCompat 分散维护 | 合并为 @niva/runtime，复用 Node 类型，并发布 @niva/types 类型包 | 用户确定整合与类型发布方向，包名由本轮命名；待实现 | 本档 D03 详细记录 |
| D04 | 通用资源与配置启动参数 | 现名 debug-resource/debug-config，并与调试判断有关联 | 改为 --resource/--config，作为正式 UI 宿主接口；来源选择与调试授权分开 | 已确定，待实现 | AR-006 |
| D05 | 应用持久身份 | 名称与 UUID 前缀组成目录 | 使用完整应用 UUID，不含名称、不截断；修复身份输入校验 | 已确定，待实现 | AR-004 |
| D06 | 关闭行为与不可用 API | 清理/退出路径不一致；调用可用性需核查 | 修复 AR-005；Bridge 不通且无有效 fallback 的 API 明确报错 | 已确定，待实现 | AR-005、本轮详细记录 |
| D07 | 调试页面统一协议代理 | 当前协议处理包内资源，debug entry 另行加载 | 评估 Wry 自定义协议代理 HTTP 页面/资源，单独处理 WS/HMR | 用户要求记录候选方案，待验证与最终定案 | AR-007 |

## 6. 初始批注

### AR-001：当前工作区不能视为已完成合并的构建基线

- 归属：R00 基线 / R13 / R14。
- 类别：基线问题；状态：已确认待决策；优先级：待定。
- 观察：2026-09-24 `git status --short` 显示 `UU Cargo.lock`；`git diff --name-only --diff-filter=U` 返回 `Cargo.lock`。同时存在新打包器及 Devtools 的已暂存改动。
- 影响：可以只读理解当前源码，但 HEAD 不覆盖所有 review 内容，依赖锁定和可构建性不能由本次盘点确认。
- 未证实：没有运行构建；不推断冲突来源、正确解决方案或其他任务进度。
- 本阶段处理：保留现场，继续架构讲解；若外部任务改变工作树，登记变化并回看受影响结论。
- 用户决定：未决定冲突处理；本任务禁止处理冲突。
- 未来验收：实现阶段另行确认干净的依赖/合并基线，并执行适用门禁。
- 后续观察（同日 22:12 +08:00）：外部活动已使 `Cargo.lock` 从 `UU` 变为未暂存修改 ` M`；未合并文件查询为空。本任务未处理冲突。初始冲突观察保留作历史记录，当前问题转为“工作树持续变化、构建基线待确认”，不再声称冲突仍存在；未运行构建。

## 7. 待核查议题（尚非缺陷结论）

| 编号 | 议题 | 归属 | 需要的证据 |
|---|---|---|---|
| Q01 | 本地、远端、iframe 与 debug 权限如何保持一致，导航后如何重新判断 | R04/R05 | 窗口创建、来源判断、WS/IPC Rust 鉴权与断连清理 |
| Q02 | 主线程、阻塞任务和流式取消的职责是否清楚 | R06/R08 | 调度代码与资源释放路径 |
| Q03 | NodeCompat 的 Native/JS 边界、同步语义和模块闭包如何解释 | R09/R10 | bootstrap、runtime、bridge、文件清单与上游 runner |
| Q04 | 新多目标打包器与既有自举/平台打包路径各服务谁，资源格式是否一致 | R12/R13 | GUI 调用点、CLI/lib、生产者与运行时读取端 |
| Q05 | 手册、公开类型、网站、方案与实际实现有哪些状态差异 | R14 | 源码与相应文档逐项对照；不凭文档日期直接判错 |
| Q06 | 兼容率、体积、CI、平台验收声明各自的范围和分母是什么 | R10/R14 | 固定版本、测试筛选/排除、实际产物及平台运行记录 |

## 8. 进度与实现阶段入口

- R00 规划与基线：已建立。
- R01–R14已完成首轮讲解并逐轮记录用户决定；R15进行中。API总表已留档，IPC范围待二次确认；其他未覆盖专题和批注保留，不因用户推进下一章而自动关闭。
- 已登记问题：按下文 AR 编号逐项维护；AR-001 交由外部合并 session、不阻断 review；其余未因讲解完成而视为修复。
- 本轮分工：`luna_deep_worker`（`/root/architecture_inventory`）只读盘点模块和调用关系；主线程制定路线、记录基线、整合档案并检查写入范围。
- 当前讨论：R15 全局回看、待决项复核与实现前收口。R01–R14已完成首轮讲解，仍有待决规则和验收缺口；不代表全部review确认或允许实现。

实现阶段的开启条件：所有章节已共同确认，未覆盖范围明确列出；所有批注都有处理结论或明确延后理由；变更范围、依赖顺序和验收条件已整理；用户明确要求开始实现。此条件满足前继续只在本档批注。仓库合并由另一个 session 负责，不作为本 review 的前置条件。

## 9. 基线文件指纹

以下清单在首次建档时生成，仅用于识别后续文件变化；不证明源码正确或通过任何运行验证。

<details>
<summary>初始关键文件 SHA-256 与 Git 状态</summary>

```text
f98d0ae745b9480f2c59af9e68dbdf39afd8d9082bf0ed68a5253ea2121184b8  Cargo.toml
2630dbfdfe8bedceb74842c8fbff60c80fcae7683456b746e7fd4b0b5fa48924  Cargo.lock
4ff98b60658eed75da6246141ab0dd92a6e207a388e698d2724844a7059490d1  package.json
cf90167a1c30455cc64008cfe23c92f3d10843f70a19b463d4f5199d825c44cf  crates/niva/src/main.rs
463adad1e1316b57c5ba618e641f2986f9a57c5f95414f5fe9376165f6b0159a  crates/niva/src/app/mod.rs
7a5ba214a1fa8f7741bc0bc88d9865467b1da3a21c6cb32be6518d11ff8b983d  crates/niva/src/app/options.rs
cd49a3c328f21aac60e2bc60dc6ce6557c58ec3ba31d1649a06ed7d59995cf74  crates/niva/src/app/custom_protocol.rs
a04770a0db0ebbe6449aeef6ad6197cdb61d0fd92e675a0a7c695a9048b7c774  crates/niva/src/app/resource_manager/mod.rs
e89986e37ed1cbb0efeb86f84baa521dcc02d8a12bdd26c02c655c00dc6fc736  crates/niva/src/app/http_server/mod.rs
866acef35f82d61eac517643ffd5b9e44be30baffdaa0e516dcb922c407af1c1  crates/niva/src/app/window_manager/builder.rs
ff851e24bd04b2460fbc8ea4dcab6a6048d07cec21c8372d7b5cc42daffcbc32  crates/niva/src/app/window_manager/permissions.rs
2edad67ec99686b94d5f4ca65fb9f34999e6d653343e77fbf95336a1f457b8bf  crates/niva/src/app/api_manager/mod.rs
d8a627fe3fa9145b2bd9d5e79252f9f64dbc8a823ee6c1fb2c168f7329c26af7  crates/niva/assets/initialize_script.js
5216774c233dc221ca20627bffeaf17551b619b7dd4f04b993964a3ae1dd7312  crates/niva/src/app/node_bootstrap.rs
8a6523f677e4e760033a1411a7c6833b2a5dce3fed6cddb90f70da384af0a262  crates/niva/src/app/node_compat.rs
520746e4b84eb52c06acfe970919a27bd376922e3df5ebc0d71edf1dcf98b27e  crates/niva/src/app/stdio.rs
52522400baf6b5a3cc0b06da42bc59fe910010c32d0a5f93c18fc58f405fc270  packages/node-compat/runtime-files.json
3555a6d6a0cddfc63c24f3c90f8dafe9ab637331de2831bfb0394d0a9ad56369  packages/types/Niva_zh.d.ts
a13992ad9cd1dc7d2a8269152f45052ae67756f05a8ddc98e3b5ae9338d57aee  packages/devtools/src/models/app.model.ts
166b14f9b808ab4f217a4e536437bc452c825ebab18df37a9b5bffbb539d7a3c  packages/devtools/src/pages/project/multi-target-build.tsx
36fb5b255e529b694534828a33d5e4c0fa0908ef8e19332aed3eccdb5e48fb53  crates/niva_packager/src/lib.rs
17ce7dd931c103ba4627cb8ff8d4ef21c59d7fba444131028bf200d3147ee2a8  .github/workflows/ci.yml
956c21151a549e2a8457b524c5de211de56980cf527ce0d2f236e8d3ea0b9309  .github/workflows/packager.yml
```

建档前已存在的变更（排除本 review 档案）：

```text
A  .cargo/config.toml
M  .github/workflows/ci.yml
A  .github/workflows/packager.yml
 M Cargo.lock
M  Cargo.toml
A  crates/niva_packager/Cargo.toml
A  crates/niva_packager/src/archive.rs
A  crates/niva_packager/src/lib.rs
A  crates/niva_packager/src/macos.rs
A  crates/niva_packager/src/main.rs
A  crates/niva_packager/src/node-compat-files.json
A  crates/niva_packager/src/resources.rs
A  crates/niva_packager/src/windows.rs
A  crates/niva_packager/tests/validation.rs
M  docs/cross-platform-packager-plan.md
A  docs/packager-usage.md
M  docs/roadmap.md
M  packages/devtools/niva.json
M  packages/devtools/src/common/ace-editor.tsx
M  packages/devtools/src/common/error.ts
M  packages/devtools/src/i18n/en_US.ts
M  packages/devtools/src/i18n/zh_CN.ts
M  packages/devtools/src/models/app.model.ts
M  packages/devtools/src/pages/project/config-editor/index.tsx
M  packages/devtools/src/pages/project/details.tsx
M  packages/devtools/src/pages/project/list.tsx
A  packages/devtools/src/pages/project/multi-target-build.scss
A  packages/devtools/src/pages/project/multi-target-build.tsx
A  scripts/check-packager-kit.py
A  scripts/check-packager-node-compat.mjs
A  scripts/create-packager-kit.py
A  scripts/packager-smoke.py
```

</details>

### 基线更新 B01 — 2026-09-24 22:12 +08:00

建档期间观察到外部工作树变化：`Cargo.lock` 不再处于未合并状态，现为未暂存修改。当前文件 SHA-256 为 `2630dbfdfe8bedceb74842c8fbff60c80fcae7683456b746e7fd4b0b5fa48924`。上面的状态清单是采集时快照，不是冻结的最终基线。下一章开始前需重新确认 HEAD 和相关文件变化。

## 10. 共同 Review 记录

### R01 / 第 1 轮：整体分层与开发期、运行期

- 用户调整：另一个 session 正在合并代码，本任务不再追踪或处理仓库不干净的问题，直接 review。覆盖此前的逐章 Git 状态复查安排；仅在具体设计证据发生变化时重新读取相关源码。
- 状态：讲解中，待用户讨论确认；不自动进入下一章。
- 本轮工作：主线程沿用上一轮 Luna 模块盘点，只读补查关键入口并组织讲解。这是与共同讨论紧密关联的证据核对，不重复派发完整盘点。
- 证据范围：2026-09-24 当前读取的源码；未运行测试、未验证平台行为、未修改代码。

| 组件 | 职责和边界 | 源码入口 |
|---|---|---|
| 业务前端 | HTML/CSS/JS 或框架产物；负责业务 UI，运行在系统 WebView 的页面环境中 | 示例项目及 `window_manager/builder.rs` |
| Rust Niva runtime | 配置、资源、窗口、事件循环、原生 API、本地通信与生命周期 | `crates/niva/src/main.rs:15`、`app/mod.rs:97` |
| Bridge | 页面与 Rust 的通信契约及实现；是 runtime 的内部组成，并非独立部署服务 | `assets/initialize_script.js`、`app/api_manager/`、HTTP/IPC 模块 |
| NodeCompat | 页面中的 Node 风格 API 适配，纯 JS 与桥接到 Native 的实现结合；不能据此推断完整 Node 或 npm 兼容 | `packages/node-compat/src/runtime/bridge.js`、`app/node_compat.rs:47` |
| Devtools | 使用 Niva 自身 API 的 React 应用，给应用开发者管理项目、配置、调试和构建 | `packages/devtools/src/index.tsx`、`models/project.model.ts:17` |
| 打包工具 | 将 runtime、项目资源、配置和平台元数据组合成可交付应用；新打包器消费预编译 runtime | `crates/niva_packager/src/lib.rs:77`、Devtools `build-scripts/` |
| types / website / examples | 开发时类型契约、文档和示例；不承担最终用户应用的原生执行职责 | 各包 manifest 与目录 |

本轮讲解链路：

1. 框架开发：编译 Rust runtime、准备兼容层和平台工具，产出供项目打包使用的运行基础。
2. 应用开发：前端项目产生网页资源，配置 `niva.json`；Devtools 帮助配置、调试和调用构建工具。
3. 应用交付：打包工具组合预编译 runtime、网页资源、配置与平台封装。跨目标打包与跨目标编译是不同工作。
4. 最终运行：用户打开应用，Rust runtime 创建窗口并加载页面；页面直接调用 Niva API，或经 NodeCompat 适配后调用 Native；Devtools 无需作为另一个常驻应用存在。

设计取舍（架构推断）：复用系统 WebView 有利于控制随包运行时体积，同时把浏览器行为差异带入平台兼容范围；预编译 runtime 降低普通前端项目打包门槛，但 Native 能力受所选 runtime 限制；NodeCompat 降低部分 Node 风格代码的接入成本，也增加语义、装载和安全边界的维护责任。此处不宣称体积或兼容率已验收。

边界提醒：JS 适配代码运行在页面环境；Rust 与 WebView 的职责区分不等于系统始终只有一个进程，具体进程模型需按平台分析。启用/关闭 NodeCompat 也不等于从可执行文件物理裁剪兼容层。

用户确认：待讨论。后续先澄清这一层组件关系，再推进 R02 启动与生命周期。

### AR-002：统一 API 与兼容环境注入的语义及文档

- 归属：R01/R09/R13/R14；类别：设计取舍及文档不一致；状态：已决策待实现；优先级：P2（容易误导配置和体积理解）。
- 证据：`docs/PROJECT_MANUAL.md` 第 4 节称 NodeCompat 是显式 opt-in、默认关闭；当前 `app/node_compat.rs:47` 的 `from_option` 对 `None` 启用默认模块集，对显式 `false` 返回 `None`。
- 另一个已观察事实：`crates/niva/build.rs:24` 从清单构造压缩资源，`app/node_compat.rs:36` 和 `:38` 通过 `include!`/`include_bytes!` 内嵌。不能再把项目模块选择直接解释成 runtime 二进制体积裁剪。
- 尚未证实：本轮没有系统核对全部配置覆盖、页面注入条件、旧/新打包路径和最终产物；这些留给 R09/R13。
- 用户决定：见下方 D01。统一 API 默认挂在 Niva 命名空间，配置选择是否注入 Node.js 兼容环境。此前笼统的“NodeCompat 默认开/关”描述不再作为目标设计描述。
- 未来验收：分别验证 Niva 命名空间 API、兼容环境注入、内嵌资源三个维度；默认/显式开关与文档一致；兼容入口复用同一实现和函数引用。配置字段名及 Node 环境注入选项的缺省值在 R09 确认。

### R01 / 第 2 轮：用户确定统一 API 与 Devtools 迁移方向

日期：2026-09-24。来源：用户本轮两条批注及补充说明。以下是用户确定的设计要求，不代表已经逐项验证当前代码符合；本轮仅修改本档案。

#### D01：统一 API 默认可用，Node.js 兼容环境按配置注入

- 所有相关 API 统一到 Niva 命名空间，例如 `Niva.fs.readFile`；该能力层默认提供，不依赖是否注入 `require` 等兼容垫片。
- 注入兼容环境后要求 `Niva.fs.readFile === require('fs').readFile`：两个入口指向同一函数，不维护两套行为或仅“结果相同”的包装实现。
- 兼容模式与非兼容模式的区别是是否注入 Node.js 的 `require`、`module`、`process` 等兼容环境及 import map。关闭兼容环境仍保留 Niva 命名空间中的统一 API。
- 用户指出直接注入这些全局对象可能有风险，因此能力提供与全局环境注入必须分开配置和解释。命名空间默认可用不构成扩大远端页面/frame 原生权限的授权；既有 Rust 权限边界仍需按 R05 核查。
- “默认注入 NodeCompat”在此指默认提供 Niva 命名空间 API，不能等同于默认注入 Node 全局环境。Node 环境注入选项的具体名称与缺省值，本轮未明确，留 R09 决定。
- 此开关控制页面环境和模块入口，不解释为二进制资源裁剪开关。
- 后续核查：API 对象/函数身份、共享状态、错误语义、初始化顺序、重复注入与页面原有模块环境的冲突，连同类型和文档一起核对。

#### D02 / AR-003：Devtools 使用新的 Node 兼容环境开发

- 归属：R08/R09/R12；类别：架构迁移；状态：已决策待实现；优先级：待实现收口排序。
- 用户要求：Devtools 文件、系统与进程操作改用 Node 风格的 `require('fs')`、`require('os')`、`process` 及 `require('child_process')`，或等价模块 import；不继续以旧 `Niva.api.fs/process/os` 作为这些能力的应用层使用方式。
- 应按职责迁移：文件操作归 fs，系统信息归 os，当前进程环境归 process，子进程启动/管理归 child_process；不能将旧 process 命名空间机械替换成 Node 的全局 process。
- Vite 等工具可能与运行时 require/module 注入冲突。若确认有冲突，按用户授权方向在 Devtools 项目构建配置中将 `import fs` 等模块引用映射到 `Niva.fs` 等统一对象，并设置所需 `process` 等页面全局对象。
- 上述环境配置仅属于 Devtools 项目/页面，不修改用户 shell 或系统全局环境变量。复用 Niva 实现，不引入另一套行为不同的 Node polyfill。
- 冲突目前是待验证条件，不宣称已经发生；选择运行时注入或构建映射的具体接入方式，留 R12 根据 Vite 开发和生产行为确认。
- 桌面专有能力（窗口、菜单等）的入口不在本次 fs/os/process/child_process 迁移中凭空推定，留 R08 核对。
- 未来验收：Devtools 的开发与生产环境均能解析模块；映射后使用同一 API；项目读取/保存、调试进程、构建子进程及错误处理正常；不再残留这些能力的旧调用路径或全局冲突。

本轮已确认上述设计方向；R01 整章尚未自动标记完成。所有实现、配置调整和原文档修订仍等整体 review 收口后执行。

### R02 / 第 1 轮：启动、配置与退出

- 用户要求：继续 review。沿用只更新档案的边界，不处理其他 session 的合并工作。
- 状态：讲解中；证据为本轮只读源码，不代表运行验证。
- 分工：主线程组织正常调用链及配置讲解；`luna_deep_worker`（`/root/startup_boundaries`）只读核查 UUID 输入与关闭清理两个异常边界。

正常启动顺序：

1. `main.rs:15` 创建 tao 事件循环，调用 `NivaApp::new`，随后 `app.run`。
2. `NivaArguments::new` 读取命令行；当前参数解析采用 `--key=value` 形式。`--stdio` 可省略值，`--build` 按是否出现识别。
3. `NivaApp::new` 先选择资源后端：提供 `--debug-resource` 时使用文件系统目录，否则使用平台包内资源。后续消费者通过统一 `ResourceManager` 接口取数据。
4. `NivaLaunchInfo::new` 读取配置：优先使用 `--debug-config` 指定文件，否则从所选资源后端读取 `niva.json`。读取当前 OS 对应的 `macos`/`windows` 配置块并覆盖公共配置，然后反序列化到 `NivaOptions`。
5. 根据 name 和 uuid 前八个字节形成应用目录标识，计算 data/cache/temp 路径。WindowManager 使用 data 路径创建 WebContext；仅计算路径不意味着此时全部目录已经建立。
6. 注册 API，创建窗口/快捷键/托盘管理器及自定义协议调度器，组装 `NivaApp`；按需启动 stdio；绑定管理器与应用；启动 loopback HTTP/WS 服务并安装外部事件处理器。
7. `NivaApp::run` 创建主窗口（ID 0），注册配置中的快捷键、托盘，然后进入事件循环。快捷键/托盘注册错误当前以日志处理，不与主窗口创建错误采用同一失败策略。

配置合并契约：`app/utils.rs:144` 的 `merge_values` 对对象递归合并，标量及数组由平台值整体替换；平台值为 null 时保留公共值，不能用 null 清空公共设置。CLI debug-devtools 为 true 时再将主窗口 devtools 强制设为 true。其他 CLI 参数是各自路径上的选择条件，不应概括成“所有 CLI 字段都会统一覆盖 JSON”。

页面入口选择：`window_manager/builder.rs:247` 起，显式 debug 以 `--debug-config` 或 `--debug-entry` 出现、且没有 `--build` 为条件。此时优先采用 CLI debug-entry，缺省再取配置 debug.entry。仅提供 debug-resource 不会自动满足这个显式 debug 条件。

| 场景 | 页面来源 |
|---|---|
| 普通打包本地页面 | 自定义协议的固定应用 origin |
| 文件系统资源或显式 debug，且 window.entry 是相对路径 | debug entry 或 loopback HTTP 作为基址，再组合 entry |
| window.entry 是绝对外部 URL | 直接使用外部 URL；权限另按 R05 核查 |

普通打包启动不会仅因配置残留 debug.entry 就连接开发服务器。Rust runtime 内资源后端直接由 debug-resource 参数选择，配置 debug.resource 与 Devtools 启动参数如何衔接留 R12 检查。

生命周期与所有权：主窗口 ID 0 是应用退出的特殊节点。正常 CloseRequested 时，若页面启用了关闭拦截，则发送 window.closeRequested 等待应用处理；否则移除窗口和 token 索引，再清理该窗快捷键、托盘、在途 API。主窗口这一路径最后设置 ControlFlow::Exit，普通子窗口不因此退出整个应用。窗口创建、Bridge 连通、页面业务就绪是不同阶段，本章不把窗口出现等同于业务 ready。

设计含义（从实现推断）：资源后端抽象使开发目录与打包资源共享加载接口；平台覆盖减少配置重复；在窗口前启动本地服务让页面初始化时可以获得有效连接信息；主窗口关联应用寿命，需要后台/托盘应用显式安排关闭行为。各管理器的更细线程和清理策略留 R06/R07。

证据入口：`crates/niva/src/app/mod.rs` 的 `NivaApp::new/run`、`NivaArguments::new`、`NivaLaunchInfo::new`；`app/utils.rs::merge_values`；`app/window_manager/builder.rs::build_webview`；`app/event_handler.rs::handle_window_event`；`app/window_manager/mod.rs::cleanup_window`。

待讨论的设计点：应用数据目录目前由 name 与 uuid 前缀共同确定，因此改名可能改变数据目录；是否希望应用身份独立于展示名称，留用户决定。null 覆盖语义与主窗口寿命规则本轮仅解释现状，不擅自调整。

#### AR-004：启动配置的 UUID 切片缺少输入校验

- 归属：R02；类别：缺陷；状态：已决策待实现；优先级：P2。
- 证据：`app/options.rs:16` 起将 uuid 定义为 String，`app/mod.rs:338-357` 反序列化后直接使用 `&uuid[0..8]`，该 runtime 路径未见先行格式/长度校验。
- 触发与影响：不足 8 字节，或第 8 字节不是 UTF-8 字符边界，会在切片时 panic，不能作为正常配置错误返回。静态结论，未运行复现；不假定 Devtools 校验可以覆盖所有直接启动路径。
- 候选方向：明确应用身份格式并在 runtime 启动边界校验；是否同时调整数据目录身份策略由用户决定。
- 用户决定：修复输入校验，目录使用完整应用 UUID（D05），不使用名称或 UUID 前缀。未来验收：无效配置可控报错；同一 ID 改名后使用同一数据/缓存/临时目录，不同 ID 隔离。

#### AR-005：关闭入口与清理失败的退出行为不一致

- 归属：R02/R06/R07；类别：缺陷及生命周期设计；状态：已决策待实现；优先级：P2，平台实际影响待验证。
- 证据：`event_handler.rs:183-193` 正常 CloseRequested 移除窗口后执行 `cleanup_window(...)?`，成功后才为主窗口设置 Exit。`window_manager/mod.rs:120-125` 先清快捷键、托盘，最后取消在途 API；前置清理出错会短路后续步骤。事件入口使用 Wait，错误记录后返回。
- 对照：`app/api/window.rs:176-187` 的主窗口 close 分支直接设置 Exit，不经过上述清理；子窗口分支经过清理。不能把所有 close 入口描述为同一条清理路径。
- 影响：用户点关闭时若前置清理出错，后续取消与主窗口 Exit 不执行；API 主窗 close 则跳过显式清理。这里确认的是控制流，不宣称已经出现进程挂住、资源泄漏或 OS 资源未回收。
- 候选方向：在 R06/R07 统一讨论关闭契约，明确必须执行的取消/退出步骤、尽力清理步骤和错误报告方式；本轮不实现。
- 用户决定：记录并修复上述清理/退出不一致问题；未要求改变主窗口生命周期政策。未来验收：OS 关闭、API 关闭、stdio 退出与清理失败路径具有明确一致的生命周期规则，并覆盖错误注入及目标平台行为。

异常边界证据由 `luna_deep_worker`（`/root/startup_boundaries`）只读返回，主线程整合为 AR-004/005；均未修改代码或运行验证。


### D03：统一页面运行时包与对外 TypeScript 类型

日期：2026-09-24。来源：用户在侧边讨论中明确要求写入 review 档案。归属：R04/R08/R09/R12/R13/R14。状态：已确定方向，待整体 review 完成后实现。本轮仅记录，不迁移文件、不调整构建、不发布 npm 包。

- **包命名**：合并后的项目命名为 `@niva/runtime`，建议目录 `packages/runtime/`；这里的 runtime 特指注入 WebView 的 JavaScript 运行时，与 Rust 原生 runtime 区分。名称为设计命名，npm scope 权限及名称可用性尚未核查。
- **整合范围**：将现有 `packages/node-compat/` 与创建/注入 Niva 对象的 `crates/niva/assets/initialize_script.js` 整合进同一个 package，统一维护页面初始化、Bridge 的 JS 端、Niva 公开 API、Node 风格实现及可选兼容环境注入。Rust 的传输、鉴权和原生 API 执行仍属于 Rust 层。
- **实现语言**：包内自有实现使用 TypeScript，构建生成供原生端注入/加载的 JavaScript。初始化脚本以该包源码为唯一维护来源，不继续手写维护另一份同功能的 init JS。合并的是项目和维护入口，最终是否需要多个装载产物按注入机制决定，不预先强制所有脚本变成单文件。
- **沿用 D01**：默认提供 `Niva.fs` 等统一 API；配置控制是否注入 require/module/process 等 Node 兼容环境及 import map。同一 API 必须复用同一实现，例如 `Niva.fs.readFile === require('fs').readFile`。包整合不改变上述模式划分。
- **直接复用 Node 类型**：通过 `@types/node` 复用 Node 模块、函数签名和相关类型，避免重新维护 fs/os/child_process 等 API 的平行签名。公开声明按实际支持的 API 选取或收窄；不能因引用完整 Node 类型就宣称未实现的 API 已可用。Niva 的窗口、菜单、Bridge 等专有能力补充自己的类型。
- **npm 类型交付**：对外发布 `@niva/types`，供其他 TypeScript 项目获得 Niva 对象及其 API 的声明和编辑器提示。类型以统一 runtime 的 TypeScript 源码与复用的 Node 类型为依据生成/导出，避免运行时和独立手写声明长期漂移。类型包仅提供声明，不负责执行页面注入。
- **类型环境边界**：Niva 命名空间声明与可选 Node 全局声明的入口应可区分。仅使用 Niva 类型的前端项目，不应被误导为运行时必然存在 require/module/process；具体声明入口和 @types/node 的全局影响在实现前 review 中核对。
- **Devtools 接入**：D02 的迁移使用此统一运行时及同源类型；Vite 的模块映射也指向同一套 Niva API，不另外引入一套 Node 实现。
- **后续验收**：统一包的 TS 检查和 JS 构建通过；Rust 注入/资源读取使用生成产物；两种环境模式及 API 引用一致性验证；外部最小 TypeScript 项目安装类型包后能正确使用 Niva API，且未实现 API 不被声明为支持；类型包版本与对应 runtime 契约明确。npm 发布在未来实现/发布阶段执行，本轮未发布。

### R02 / 第 2 轮：通用 UI 宿主接口、应用身份与调试代理

日期：2026-09-24。来源：用户本轮批注与补充问题。仅记录设计和后续验收，不进行实现。D03 及其他 session 已写内容保留。

#### D04 / AR-006：resource 与 config 是正式启动接口

- 用户决定：`--debug-resource` 改为 `--resource`，`--debug-config` 改为 `--config`。这是将 Niva 当作纯 UI 窗口/宿主使用的正式能力，不仅用于调试。
- 状态：已决策待实现；归属：R02/R05/R11/R12/R13。
- 重要连带调整：当前 `debug_config` 的存在参与 explicit_debug 判断。改名时必须拆开“外部配置/资源来源”与“启用调试页面及授权”的语义，不能使正式 `--config` 自动打开调试权限。
- 后续范围：Rust 参数与来源选择、Devtools 启动命令、stdio 示例、脚本、类型/文档同步更新；按项目规则不保留旧参数兼容别名。
- 验收：无需打包即可以 `--resource`/`--config` 运行 UI；单独指定配置不授予额外调试权限；包内默认路径仍可工作。

#### D05：以唯一应用 ID 定位数据

- 用户明确：ID 才是唯一身份，展示名称不参与持久目录匹配。沿用当前 UUID 作为应用 ID，目录使用完整 UUID，不继续截取前八个字节。
- data/cache/temp 的平台根目录规则与子目录身份分开；各平台根目录下以完整 ID 定位。改名且 ID 不变时，仍能访问原有同 ID 数据。
- AR-004 同时修复身份格式/路径安全校验，错误应正常返回。未来实现不以任意未校验字符串直接作为路径。
- 此决定不承诺自动迁移历史 name_uuidPrefix 目录；用户未要求迁移，遵守仓库不添加兼容迁移的规则。

#### D06：统一关闭与 API 不可用行为

- AR-004/AR-005 均记为必须修复、待实现；本轮“记录要修复”不改变整个 review 完成后才实现的约定。
- 对包内页、开发服务器页面、外部 URL，API 若 Bridge 不通且不存在可用 fallback，应明确报错，不静默成功、不无限等待。
- fallback 必须实际支持该 API 的语义并满足当前权限；权限拒绝不视为可以切换通道绕过的连接失败。不据此新增兼容分支或要求所有 API 必须有 fallback。
- R04/R08 继续明确同步抛错、Promise rejection、流错误的对应契约，以及连接初始化/断线后的有界等待和错误码；本轮未凭空指定错误码。

#### D07 / AR-007：debug URL 经 Wry 自定义协议代理的候选方案

- 用户目标：减少调试时页面来源、Bridge、WS/XHR 与 Vite 的配置负担。将调试服务器视作开发资源后端，让页面仍经统一应用协议加载。
- 命名说明：用户本轮称 debug-url，当前源码名为 debug-entry；是否统一改为 `--debug-url` 尚未明确，记录该概念，不提前改名。
- 状态：待验证的设计方案；已授权记录，未批准切换默认实现。归属：R03/R04/R05/R12。
- HTTP 页面/资源候选链路：页面访问 Niva 自定义协议 → Rust 协议处理器 → 配置的开发服务器 → 返回响应。当前 `custom_protocol.rs` 处理 ResourceManager 资源，不把本方案描述成已交付 HTTP 反向代理。
- WebSocket 可以代理，但 Wry 自定义协议的请求/响应回调不是 WebSocket 双向连接通道。候选为：页面使用 `ws://127.0.0.1:<port>` → Rust 独立 WS 代理端点 → Vite HMR 或指定上游 WS。Niva 原生 Bridge WS 与 HMR WS 必须分开路由和鉴权。也可评估 HMR 直接连接开发服务器的方案，但不能误认为普通资源代理自动涵盖 HMR。
- CSP 正确指令名是 `connect-src`，它控制 fetch、XHR、WebSocket 等连接；不是 content-src。只放行本会话需要的 loopback HTTP/WS 和明确的开发地址，并核查脚本、样式等各自策略。新增更宽松 CSP 不能抵消另一条已有严格策略，HTTP 头与 HTML meta 必须一起考虑。
- CSP 放行只是必要条件之一；直接跨源 XHR/fetch 仍涉及 CORS/预检/凭据，WS 涉及握手 Origin 与 token，不能仅靠改 CSP 承诺“保证可访问”。跨平台应用 origin 不同，要分别验证。
- 代理会让开发服务器内容在应用 origin 中执行，应限定为明确配置的开发源，检查重定向目的地及代理路由边界，避免把任意外部网页转成有完整 Native 权限的本地页；保持 Rust 鉴权。
- 待验证细节：Vite 的模块绝对 URL、import.meta.url、HMR 主机/端口/协议和重连；HTTP 方法/请求体/响应头/MIME/重定向；Cookie 与浏览器存储语义；请求取消、超时、资源大小和流式限制。自定义 scheme 的 XHR/POST 行为不能仅凭 Rust Request 类型认定跨平台可用。
- 未来验收：macOS/Windows 真机分别验证页面、模块和资源加载、热更新与重连、本地 XHR/原生调用、CSP/CORS 错误可诊断；断开 Bridge 且无可用 fallback 时正确报错；不要求用户手工关闭全局安全策略。验证通过后再决定方案，而不是仅看到首页可加载即视为完成。

本轮资料依据（2026-09-24 查阅，未运行原型）：

- Wry 0.57.0 自定义协议 API：请求/响应接口及跨平台 Origin 差异。https://docs.rs/wry/0.57.0/wry/struct.WebViewBuilder.html#method.with_asynchronous_custom_protocol
- Vite 官方 server.hmr 文档：反向代理需支持 WebSocket，并可配置 HMR 连接信息；连接代理失败存在直连行为。https://vite.dev/config/server-options#server-hmr
- MDN connect-src：覆盖 fetch、XHR 与 WebSocket，部分浏览器不能仅靠 self 匹配 WebSocket。https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Content-Security-Policy/connect-src

可行性结论：HTTP 资源代理与独立 WS 代理从接口/协议分工看可作为实现方向；“完整透明支持所有调试请求”尚未验证。主线程完成了针对性官方资料核对和档案记录，没有修改代码、配置或其他文档。


#### D07 / AR-007 补充：动态端口方案与 macOS 隔离实测

日期：2026-09-24。来源：侧边讨论中，用户要求实际测试 Wry 协议页面访问 Niva server 的 WS/XHR，测试通过后明确要求将方案写入 review 档案。本补充更新上文“未运行原型”的证据范围，不覆盖其他 session 已记录的设计。状态：连接基础已获 macOS 实测证据；完整调试代理仍待实现和验收，不切换默认路径。

**拟采用的分工与动态配置**

1. 开发页面及 HTML/JS/CSS 等资源经 Niva 自定义协议加载，Rust 协议处理器转发到明确配置的开发服务器；保持应用页面 origin。此代理尚未实现于本次实验。
2. Niva server 先绑定动态 loopback 端口，Rust 取得实际地址后，再创建 WebView 并注入 Bridge 所需 HTTP/WS 地址与窗口凭据。应用代码无需知道端口，配置文件不持久化端口或 token。
3. 调试代理生成页面响应时，根据本次端口生成精确 connect-src，例如 `connect-src 'self' http://127.0.0.1:55125 ws://127.0.0.1:55125`，再按需要加入明确的 Vite HMR 地址。无需放行所有 loopback 端口或关闭 WebView 安全检查。
4. 对已有 CSP 响应头与 HTML meta 统一处理；追加宽松策略不能放宽已有严格策略。动态响应头生成属于待实现方案，本次实测使用页面动态插入 CSP meta。
5. 原生 API 沿用本地 WS 与同步 HTTP/XHR，保持 Rust 侧窗口 token、准确 origin 和方法权限检查；XHR 继续返回匹配 origin 的 CORS 许可。API 调用身份不能因资源代理而失去校验。
6. Vite HMR 初步优先评估显式地址直连；确需转发时，在本地网络服务设置独立 WS 代理路由。Wry 自定义协议请求/响应处理器本身不承载 WebSocket 双向代理。HMR 路由与 Native Bridge 路由分开。
7. 只将明确授权的开发服务器内容代理进应用 origin，代理重定向与路由边界也需校验；不能将任意外部页面自动升级成可信应用。

**实验方法与产物身份**

- 平台：Darwin arm64；WebView 中 os.info 返回 Mac OS 26.6.2、aarch64。
- 使用现有 `target/release/niva` 的副本构造临时 `.app`，未重新编译，因此证据绑定该二进制，不推断与并行变化的最新源码完全一致。
- 被测二进制 SHA-256：`86dc0a104cef1ad19971d3edeeb53af435781b7f90af842cbc4480e144c5ee44`。
- 测试应用：`/tmp/niva-protocol-connect-probe/Probe.app`；页面实际地址为 `niva://app/index.html`，origin 为 `niva://app`。启动只传 `--stdio`，没有以 debug-resource 绕回普通 HTTP 页面。
- 页面从已注入地址取得实际端口，添加动态 CSP meta 后，新建独立原生 WebSocket，并发起同步 XMLHttpRequest；不以 CSP 设置前已建立的 Bridge socket 作为新 WS 可连的证据。
- 两条连接均实际调用只读原生方法 os.info，检查返回值；额外检查 Niva.callSync 包装、CSP 拒绝未放行地址及错误 token 的 WS 连接失败。
- 原始结果：`/tmp/niva-protocol-connect-probe/results.json`；可复跑脚本：`/tmp/niva-protocol-connect-probe/run.py`；页面：`/tmp/niva-protocol-connect-probe/index.html`。临时目录可能被系统清理，关键方法、二进制身份和结果已内联保存于本档。

| 检查 | 第一次启动：55125 | 第二次启动：55137 |
|---|---|---|
| 页面 origin | niva://app | niva://app |
| 动态 CSP 后新建 WS，携带窗口 token 调用 os.info | code 0，结果正确 | code 0，结果正确 |
| 同步 XHR POST /__niva_sync，携带窗口 token 调用 os.info | HTTP 200，API code 0 | HTTP 200，API code 0 |
| Niva.callSync('os.info', []) | 返回正确系统信息 | 返回正确系统信息 |
| 访问未放行的 http://127.0.0.1:1/not-allowed | 捕获 connect-src 违规，访问失败 | 捕获 connect-src 违规，访问失败 |
| 同一 WS 地址使用故意错误的 token | 连接失败，未触发 onopen | 连接失败，未触发 onopen |
| stdio EOF 后进程结束 | exit 0 | exit 0 |

**结论与未覆盖范围**

- 已证实：在上述 macOS 二进制和测试页中，Wry 自定义协议 origin 可以访问动态端口的 Niva WS 与同步 XHR；精确动态 CSP 与有效凭据可以同时工作。两次端口变化无需修改静态项目配置。
- 未覆盖：Windows/WebView2、实际 Vite HTTP 资源代理、HMR 直连/代理及重连、严格原始 CSP 响应头的处理、重定向、Cookie、完整 iframe/导航权限、流式/大数据或所有同步 API。
- 本次页面用 inline script 执行测试，script-src 含 unsafe-inline；不将这份测试策略作为最终产品 CSP，也未证明现有多条 CSP 可以被动态 meta 放宽。
- 错误 token 负例观察的是浏览器 WS 连接失败，未单独捕获握手 HTTP 状态；未运行错误 token XHR 负例。
- 本轮仅在临时目录构造并运行实验，随后按用户要求更新本档；未修改仓库源码、配置、打包逻辑或默认启动路径。后续实现仍受整体 review 收口约定约束。


#### D07 / AR-007 补充：Windows WebView2 真机验证通过

日期：2026-09-24。来源：用户在侧边讨论明确告知 Windows 已连接并要求验证。本节补齐此前连接实验的 Windows 覆盖；不将完整 Vite 代理/HMR 方案标记完成。

**被测版本与隔离方式**

- 设备：SSH 别名 windows，主机 Bramblex；Windows 11，系统版本 10.0.26200，AMD64。真实 WebView2 页面 UA 包含 Chrome/153.0.0.0、Edg/153.0.0.0。
- 设备已有候选运行时未包含 __niva_sync，因此没有使用旧版本代替当前功能验证。将本机当前源码快照复制至 Windows 临时目录独立构建，不修改任一现有仓库或既有产物。
- 源码快照：本机 `/tmp/niva-protocol-connect-probe/windows-source.zip`，SHA-256 `b8dcff1a79c22a38f3c6bab0a2fbc1bd72953e64100bd1e8c5d9f59d29f060ad`。
- 临时工作目录：`C:\Users\brambles\AppData\Local\Temp\niva-protocol-probe-20260924`；源码在其中 source 子目录。
- Windows 原生编译：`cargo +1.98.1 build --release --locked -p niva` 成功，耗时 1m30s，报告 17 项 warning。未做修复；本次是连接实验所需构建，不是完整 Rust 门禁或发布验收。
- 裸 runtime SHA-256：`b3be08f0996ebb213b7891f417e2afa5894cd630adc9fb9d55bc353749f1b466`。
- 使用既有 win_packager 的临时副本嵌入测试页面和配置；测试产物 `NivaProtocolProbe.exe` SHA-256：`031d13ed9027bd6a13e09df7d70947545a47bcfcd111b3d81d83f96b846fcb3f`。

**启动与实际结果**

首次通过 SSH 非交互会话直接启动，WebView2 在页面创建前报 HRESULT 0x80070578（无效窗口句柄），不能据此判断 WS/XHR。随后通过一次性、普通权限 Interactive 登录任务在已登录桌面会话运行，同一测试二进制连续两次通过。该临时任务完成结果为 0，已移除，测试进程已退出；初次失败日志单独保留为 ssh-*，没有覆盖或当作通过结果。

页面由 Wry 自定义协议提供，在 Windows 实际地址为 `http://niva.app/index.html`、origin 为 `http://niva.app`。没有传 debug-resource/debug-entry；与 macOS 一样，动态 CSP meta 生效后才新建被测 WS，并发起同步 XHR。

| 检查 | 第一次桌面运行：57432 | 第二次桌面运行：54569 |
|---|---|---|
| 新建 WS，携带有效窗口 token 调用 os.info | API code 0，返回 Windows/x86_64 | API code 0，返回 Windows/x86_64 |
| 同步 XHR POST /__niva_sync 调用 os.info | HTTP 200，API code 0 | HTTP 200，API code 0 |
| Niva.callSync('os.info', []) | 返回正确系统信息 | 返回正确系统信息 |
| CSP 阻止未放行的 127.0.0.1:1 地址 | 捕获 connect-src 违规，访问失败 | 捕获 connect-src 违规，访问失败 |
| 同一 WS 地址使用错误 token | 连接失败，未进入 onopen | 连接失败，未进入 onopen |
| stdio EOF 后退出 | exit 0 | exit 0 |

**证据与结论更新**

- 本机结果副本：`/tmp/niva-protocol-connect-probe/windows-results.json`；桌面执行日志：`/tmp/niva-protocol-connect-probe/windows-desktop-run.log`；runner：`/tmp/niva-protocol-connect-probe/windows-run.py`。
- Windows 原始结果位于上述临时工作目录的 windows-results.json，页面/runner/日志也保留于该目录供复查；临时路径不保证永久保留，关键证据已内联于本档。
- 现已分别在 macOS 系统 WebView 和 Windows WebView2 验证：固定自定义协议页面 origin + 动态 loopback 端口 + 精确 CSP + 窗口 token，可以完成 WS 与同步 XHR 原生调用。
- 两平台使用的二进制身份分别记录，不宣称是完全相同源码版本的发布矩阵。此处是有界连接可行性证据。
- 仍未覆盖：真实 Vite HTTP 资源代理、HMR 直连/代理与重连、CSP 响应头与 meta 冲突处理、完整 iframe/导航/重定向权限、错误 token XHR、流式/大数据与全部同步 API。Windows 未覆盖范围从“没有真机实验”收窄到上述具体项目。
- 本轮未修改源码或默认配置；仅独立构建运行测试，并补充 review 档案。完整方案实现仍等整体 review 收口。

### D08：IPC fallback 收敛为异步、单次、JSON 调用

日期：2026-09-24。来源：用户提出 IPC fallback 的范围并询问合理性。评审建议：采纳该边界；状态：范围提案已记录，具体 API 清单及错误契约待 R04/R08 确认，未实现。

- 适用条件：页面直载时 WS 受限，可考虑平台 IPC；不将其表述为全局唯一方案，D07 的受控开发资源代理仍独立保留。
- 能力边界：仅支持异步、单次请求/响应、可明确 JSON 表达的参数和结果，并满足每窗口/来源/API 的 Rust 授权。
- 拒绝对外流式/持续任务接口；文件与网络的一次性异步操作必须提供真正可用的 unary fallback。已有实现依赖流式 handler 不能成为拒绝整个功能的理由，应在 Native 层提供有界的一次性操作，尽量复用底层 I/O 实现。不得无界缓存流或将持续订阅伪装成一次性响应。
- 拒绝跨 IPC 传输的二进制入参和结果，包括 ArrayBuffer、TypedArray/DataView、Buffer 等；不能通过 JSON.stringify 静默变成对象，也不为此引入 Base64 或数字数组转运。对已声明为二进制的 API 应在执行副作用前拒绝，不能仅等响应序列化时发现。
- 拒绝需要跨 Native Bridge 的同步接口。纯 JS 的同步计算（例如 path.join）不需要 Bridge，仍可正常工作；拒绝范围不是所有同步 JavaScript 函数。
- 统一 Niva/Node 入口继续共享实现和能力判断；接口存在不代表该页面所选传输支持全部行为。Node 的 fs.readFile 默认 Buffer 结果仍属于二进制受限情形；指定文本编码的异步读取应作为必做 fallback，而非以现有 handler 是流式为由拒绝。
- 明确报错：同步 API 同步抛错；Promise API reject；回调 API 经错误回调返回，不能静默成功或悬挂。具体错误码待统一 runtime 契约确定。
- fallback 不绕过权限，尤其不能把 WS 的拒绝授权当作切换 IPC 的理由。已发出的有副作用请求在连接丢失后结果不明时不得自动经 IPC 重放，避免重复写入/重复启动进程。
- 后续区分“持续订阅/监听”与基础窗口事件转发的范围，不能仅凭“禁止流式 API”顺便删除窗口事件能力。
- 沿用有限报文、超时和并发限制；文件与一次性 HTTP 网络操作是必做范围，窗口操作、剪贴板文本和系统信息也逐项核查。限制值在实现评审时按实际用途确定，不直接把现有 256 KiB 当作最终产品上限。

本轮源码依据：`crates/niva/src/app/api_manager/mod.rs::ipc_call` 当前只接收 ClientMsg::Call 和 HandlerKind::Unary，有 Rust 来源/API 权限检查、256 KiB 请求/响应限制及并发/超时约束。`initialize_script.js` 的 streamCall/callSync 已拒绝无本地 WS 能力的页面。仍未证实所有公开 API 对嵌套二进制值的预检，以及未来统一 Node 风格包装均遵守本边界。

当前传输选择按页面能力决定；不把这些已有限制误写成“任何本地 WS 失败都会自动转 IPC”。本轮只读核查与追加 review 档案，没有修改源码、运行测试或变更既有平台实验记录。

#### D08 补充 — 2026-09-25：文件与网络必须具备可用 fallback

用户明确：文件、网络这些基础能力肯定需要实现可用 fallback。此前将 IPC 范围倾向于窗口、文本和少量元数据过于保守，按本条修正。

- 必做文件范围：异步文本读写、目录列举、文件/目录元信息，以及创建、重命名、删除等一次性操作，具体签名在 R08 对照统一 API 清单确认。
- 必做网络范围：一次性异步 HTTP 请求（含常用方法、请求头、文本/JSON 请求体），返回状态码、响应头和有大小限制的文本/JSON 响应。由 Rust 执行网络 I/O，页面仅经 IPC 传入请求并取回结果，不依赖页面 fetch/XHR 能跨源连通本地服务。权限、目标地址和重定向规则仍在 Native 层执行。
- 纯粹因现有 fs/http 实现依赖 WS 或流式 handler 而让这些操作不可用，不满足“够用”的要求。统一 API 下实现传输适配，不要求调用方手工更换业务接口；应尽量复用 I/O 逻辑与错误转换。
- 继续保留此前的限制：不提供跨 Native 的同步调用、持续流/订阅、原始二进制参数和结果；TCP/WS/SSE 持续会话不属于本轮承诺的一次性 HTTP fallback。Node http.request 的流式对象契约不能用一次性 JSON 结果冒充，具体网络高层入口留 R08 确定。
- 具有副作用的操作，尽可能在执行前完成参数、权限与大小预检；读取/网络响应在处理过程中执行大小上限和超时，超限明确报错而非无限缓存或静默截断。结果未知时不自动重放请求。
- 本轮 commentary 提到“内部可用编码”只是潜在实现方向，不是用户已经批准放开二进制限制。Base64 等二进制转运仍不纳入已定范围；若后续用户要求二进制读写/下载，再单独决定编码、落盘结果或其他契约，不能擅自改变 D08 的已定边界。
- 验收要求：在 WS 不可用、只有 IPC 的授权页面，实际完成文本文件读写、目录操作和 HTTP 文本/JSON 请求；验证错误、超限、超时、拒绝权限与关闭清理。仅确认存在 IPC 通道不足以验收。

本轮只更新 review 档案，没有实现 fallback 或修改源码。

#### D08 实现范围表 — 2026-09-25

用户要求将上一轮清单写入档案。下表记录后续实现范围，非当前已交付能力；上一轮标为“建议实现”的两项继续保留建议状态。

| 能力 | 范围决定 | 契约 |
|---|---|---|
| 文本文件读取、写入、追加 | 实现 | 异步，指定文本编码 |
| 文件/目录信息、存在性、目录列表 | 实现 | JSON 元数据 |
| 创建目录、重命名、移动、删除 | 实现 | 一次性异步操作 |
| 文件复制 | 实现 | Native 内部复制，内容不经过 IPC |
| HTTP/HTTPS 请求 | 实现 | 常用方法、请求头、文本/JSON 请求体 |
| HTTP 文本/JSON 响应 | 实现 | 状态码、响应头、有界内容 |
| 下载到文件、从文件上传 | 建议实现 | Native 传输；IPC 仅传路径、参数和最终结果，无进度流 |
| 系统信息 | 实现 | 已授权的一次性 OS、路径、环境等查询 |
| 窗口操作、对话框、剪贴板文本 | 实现 | 逐项授权，异步简单结果；具体方法清单留 R08 |
| 有界子进程执行 | 建议实现 | 等待退出，有限文本输出及退出码 |
| 文件二进制读写经 IPC 返回/传入字节 | 不实现 | 包括默认返回 Buffer 的 readFile；文本编码版本支持 |
| HTTP 二进制请求体/响应体跨 IPC 传输 | 不实现 | ArrayBuffer、Buffer、TypedArray 等 |
| 文件流和文件监听 | 不实现 | createReadStream、watch 等 |
| 网络流、SSE、TCP/UDP、WebSocket 会话 | 不实现 | 持续连接、分段数据交互 |
| 交互式子进程、实时输出 | 不实现 | 持续 stdin/stdout 会话 |
| 需要 Native 的同步调用 | 不实现 | readFileSync、execSync 等 |
| 纯 JS 方法 | 照常可用 | 不需要 IPC，例如 path.join |

复制和候选的上传/下载允许 Native 内部处理二进制文件，并不开放 IPC 字节传输。一次性 HTTP 高层接口不能冒充 Node http.request 的流式对象契约。所有支持项仍受权限、超时、并发与大小限制，最终阈值待对应章节确定。

### R03 / 第 1 轮：资源后端、页面地址与通信服务

日期：2026-09-25。状态：讲解中；仅静态源码核对，本轮没有运行测试或应用。主线程讲解协议与加载链；Luna 只读核查资源打包/读取对应关系。

先区分三个对象：应用静态资源（HTML/CSS/JS/图标等）、Rust 内嵌页面运行时资源（当前 NodeCompat 等，未来 D03 统一 runtime）、用户磁盘文件。应用资源后端抽象不等于授予任意磁盘读取权限；`__niva_fs` 是单独的凭据保护入口。

- `ResourceManager` 统一 exists/load/load_capped/extract/load_icon 等操作，目录后端与应用包后端各自实现。
- 页面加载地址与资源物理位置分开。打包本地页以固定应用 origin 访问：macOS `niva://app`，Windows WebView2 映射为 `http://niva.app`。前端引用相对路径，协议处理器把 URL 映射成资源键，再交给读取后端。
- 本地 HTTP/WS 服务使用动态 loopback 端口，承担 WS、同步调用和文件 URL 等接口；打包模式普通静态 HTTP 路由关闭。当前文件系统开发路径仍可走 HTTP 静态路由，D04 参数改名与 D07 开发代理均未因此宣称已实现。
- 固定页面 origin 的设计目的，是将浏览器页面身份与通信端口解耦。浏览器数据目录仍受应用 ID 影响；“origin 固定”不等于任意身份/目录切换都自动保留数据。

一次包内页面加载的具体步骤：

1. WebView 请求应用 origin 下的 `/`，`asset_path` 映射为 `index.html`；目录末尾 `/` 也追加 index.html。
2. `response_for_request` 验证方法（目前 GET）、目标和路径，解码并拒绝不合法路径；`__niva_` 保留前缀只允许已授权的 compat 资源路径。
3. 普通资源从 ResourceManager 读取；`__niva_compat/` 从 Rust 内嵌资源按模块 allowlist 提供。
4. 从路径推断 MIME 类型；对识别为文档导航的 HTML 执行当前 NodeCompat 注入和 CSP meta 调整。普通 fetch HTML 不应被一概当作页面导航重写。
5. 返回状态、Content-Type、nosniff 和正文，WebView 继续加载 JS/CSS/图片。初始化 Bridge 的脚本另由窗口 builder 通过 Wry 注入，不等同于 HTML 中的 compat 资源标签。

当前处理限制：`custom_protocol.rs` 中为 4 个 worker、32 个排队任务、15 秒响应超时、32 MiB 单响应上限；完整正文在内存中组装，GET-only。队列满返回 503，超时返回 504，超限返回 413，资源找不到返回 404。上述数字仅是当前实现参数，不是用户已批准的最终产品指标；也不能因有 async responder 就认定为流式 HTTP 服务。视频 Range、HEAD、超大资源等需求留后续资源/网络契约审查。

磁盘文件入口：`http_server/mod.rs` 的 `__niva_fs` 使用窗口 token 查找窗口，并控制 CORS；与应用资源根目录加载是两条路径。其路径权限、错误页和 iframe/导航边界留 R05 逐项核查，不以存在 token 就概括为安全验收完成。

CSP 的当前处理范围：`patch_document_csp` 调整我们持有的 HTML 中 CSP meta，加入当前 loopback 地址，并为需要的兼容脚本处理 nonce。它不代表可以改写直接从外部 URL 加载的服务器响应头；D07 的真实开发服务器代理仍须单独实现/验证。

本轮源码入口：`crates/niva/src/app/custom_protocol.rs::response_for_request/asset_path/patch_document_csp`、`window_manager/builder.rs::build_webview`、`http_server/mod.rs` 的 WS/sync/fs/static 路由。未新增设计决定；本章待用户讨论确认。

R03 资源封装补充（Luna 只读证据，主线程整合）：

- `niva_packager/src/resources.rs::prepare` 将配置作为 niva.json 收入资源，各文件字节拼接为整体；JSON 索引按相对路径记录 `[offset, length]`，偏移对应解压后的数据。整体采用一个 Deflate 压缩流，不是逐文件独立压缩。
- `AppResourceManager::new` 启动时解压整体资源数据；`load` 按索引切片，使用 checked_add 与范围检查。因而单次协议响应 32 MiB 限制不等于启动时整个资源包只占 32 MiB，也不代表按需逐文件解压。
- macOS `assemble` 写入 `.app/Contents/Resources/RESOURCE_INDEXES` 和 `RESOURCE_DATA`，运行时从可执行文件相对位置读取。Windows `assemble` 写入 EXE 的同名 RT_RCDATA 资源，运行时从当前模块读取。生产者与消费者格式/平台落点表面一致，未运行验收。
- 目录资源后端对路径解码、规范化并验证未逃出资源根。打包 collect/safe_file 拒绝符号链接与异常路径；不据此推断所有并发文件系统变动场景安全。

#### AR-008：打包输入并发替换的条件性风险

- 归属：R03/R05/R13；类别：待核实安全边界；状态：待核实；优先级：待定。
- 证据：Luna 盘点 `niva_packager/src/resources.rs::safe_file` 先校验符号链接等属性，调用方随后 fs::read，检查与读取不是同一个文件句柄上的操作。
- 条件：只有当资源目录在打包期间可能被不可信并发写入者替换时，才涉及该竞态威胁；未验证可利用性，不宣称已发生越界读取。
- 后续：R13 确认输入目录信任模型与打包一致性要求，再决定是否需要基于已打开句柄的校验。当前不修复，也不将其默认提升为发布阻断。

### OPT-01：代码整体压缩、媒体独立存储的混合资源方案（最终优化项）

日期：2026-09-25。来源：用户在侧边讨论中明确要求将方案记入 review，并指定为最后的优化方案。归属：R03/R13/R14。状态：方案已记录、未实现；在整体 review 收口、必做功能与正确性修复完成并通过验收之后，再评估和实施，不作为当前架构 review 或功能实现的前置任务。

**目标与基本结构**

保留代码文本整体压缩的收益，同时避免应用启动时将全部媒体解压到内存。代码和媒体是同一资源容器中的逻辑分区，不要求新增两个外部分发文件；继续兼容 macOS 应用包与 Windows 单 EXE 的产品形态。

| 资源类别 | 打包方式 | 运行时读取 |
|---|---|---|
| HTML、JS、CSS 及必要的小型配置文本 | 合并为一个压缩块，内部索引记录各文件偏移与长度 | 启动时一次解压，随后按索引读取 |
| 图片、音频、视频等媒体 | 每个文件一个独立条目 | 访问时只读取该条目 |
| 已压缩媒体，尤其大视频 | 独立条目，默认采用 Stored（不重复压缩） | 保留直接定位字节范围的能力，避免为了读取尾部而解压前面内容 |
| 压缩收益明确的其他资源 | 独立压缩条目 | 访问时单独解压 |
| 大型 JSON 数据集、文档等非启动必需文本 | 归入独立按需资源，不因文本扩展名自动放入代码块 | 访问时读取/解压 |

**容器与接口方向**

- 优先评估标准 ZIP 作为外层容器：代码组合块作为一个条目，媒体作为各自独立条目。ZIP 中不同文件条目本身不共享压缩字典，因此代码组合块仍需内部文件索引。
- 标准 ZIP 是候选具体格式；先用完整 release 体积和读取成本比较，再确定。若需沿用现有资源容器，也必须实现相同的逻辑分组和按需读取能力，不并行维护两套长期格式。
- ResourceManager 上层继续按逻辑路径读取，前端资源 URL 不因内部布局改变。启动只读取必要索引并解压代码块，不整体展开媒体；资源读取后端应避免把整个媒体容器先复制到堆内存，从而抵消按需读取收益。
- 媒体条目独立存储只提供随机访问基础；视频拖动、Range/HEAD、响应分段与取消仍需资源服务层配套实现和平台验收，不能宣称换格式即可自动支持。
- 默认不要求媒体解压落盘；如引入缓存，必须设置总量上限并用实测证明必要性，避免逐步缓存成整包常驻。

**取舍和范围**

- 代码块仍在启动时全部解压，启动成本取决于代码块大小；这一轮不进一步引入首屏/懒加载代码分组。
- 媒体不参与文本共享字典，容器元数据也有开销，最终包体积可能比现有整包压缩增加；不预设收益数值。
- 侧边讨论中 ZIP 读取代码增量的 50–200 KB、预留 300 KB 仅是未实测的工程估算，不能用作 3.3 MB 门禁通过证据。
- 新格式需同步生产者、读取端、格式版本和损坏输入校验，按仓库规则不添加旧格式兼容回退。不得在优化任务中删除或迁移用户数据。

**实施顺序与验收**

1. 先完成 review 确定的必做功能、正确性/权限修复及其验收；此项排在最后。
2. 冻结同功能的优化前基线，选择代码为主、图片为主、大视频为主的代表性应用资源。
3. 比较现有整包压缩、标准 ZIP 逐文件方案与本混合方案，记录最终分发大小、完整 Native release 大小、启动到页面就绪时间、峰值内存和资源读取延迟。
4. 在 macOS、Windows 分别验证相同逻辑路径、HTML 注入、媒体按需读取及资源服务行为；并发、损坏索引、截断压缩数据和解压大小限制不得破坏正确性。
5. 按仓库门禁测量包含全部已确定 Native 功能、内嵌页面运行时与索引开销的完整 release 主程序，确认小于 3,300,000 bytes；不以独立 ZIP 样例体积代替。

本条只建立最终优化方案，不启动实验、不修改代码、依赖、配置或其他文档。

### R04 / 第 1 轮：Bridge 调用链、请求所有权与终止

日期：2026-09-25。状态：讲解中。主线程阅读 JS transport 和窗口/HTTP 路由；Luna 只读核对 Rust 帧协议与取消机制。本轮不运行验证、不改源码。

Bridge 分为三层：公开 API/Node 风格适配；JS 的请求、Promise、事件与传输管理；Rust 的鉴权、方法分发、任务及资源管理。D01/D03 要统一前两层的维护入口，不取消 Rust 层的独立职责。

一次正常 WS 调用：

1. 窗口创建时生成窗口 token，受信任页面获得初始化连接信息。同源 iframe 可读取顶层凭据，但各自建立连接。
2. HTTP 服务先用路径、Host、token 与精确 Origin 检查握手；第一帧 hello 再校验协议版本和窗口 ID。Rust 为连接分配 connection ID，不能仅信任 JS 自报的 wid。
3. JS 分配 request ID，保存对应 Promise/回调，发送 call 消息；Rust 按方法找到 handler 并调度执行。
4. result 带原 request ID 回到对应连接；JS 找到 pending，成功 resolve、失败 reject，并移除记录。
5. 任务的完整身份为 `(window ID, connection ID, request ID)`。不同窗口或 iframe 的相同 request ID 不会因此成为同一任务；一条连接断开不应取消同窗其他连接的任务。

公开传输形状（v1）：hello、call、result、event、cancel 为 JSON 文本帧。call 携带 id/method/args；result 携带 id/code/message/data。请求事件带 id，无请求 id 的事件用于窗口级通知。二进制帧另外承载字节，带版本、flags、请求 id、序号；START/END 表示分组边界，不等同于整个 API 完成，整个调用以 result 结束。

流式任务除了最终结果，还需要保存输入通道、事件/数据回调、取消信号及 Native 资源。它不是简单的“调用多次普通 API”。因此 D08 中 IPC 只实现一次性 JSON 子集，WS 承担持续流与二进制，而同步 Native 调用当前另走同步 XHR，不是同步 WebSocket。

生命周期：用户取消、连接关闭、窗口关闭、服务端超时都可能结束任务。JS 当前在 WS close 时拒绝 pending、清空发送队列并延迟重连；重连获得新的连接身份，不恢复或重放旧任务。服务器超时不能覆盖还在浏览器发送队列里、尚未发出的调用。取消也不等于回滚已发生的文件/进程副作用，具体 handler 的终止与释放留 R06/R08 核查。

IPC 当前有独立 pending 表与回复超时，窗口/页面来源由平台侧获取并在 Rust 检查；流式、原始二进制和同步范围按 D08 收敛。它目前按页面能力选择，不等于本地 WS 任何失败都会自动切换 IPC。

源码入口：`crates/niva/assets/initialize_script.js::sendIpcCall/connectSocket/streamCall/handleTextMessage/handleBinaryMessage`；`http_server/mod.rs::ws_pump_inner/parse_hello`；`api_manager/mod.rs`；`api_manager/protocol.rs`；`window_manager/window.rs` 的连接表。

#### AR-009：JS 显式取消没有结束对应 Promise

- 归属：R04/R06；类别：缺陷；状态：已确认待决策；优先级：P2。
- 证据：`initialize_script.js::streamCall` 返回对象的 cancel 删除 pendings 项并发送 cancel，但不调用 pending.reject/resolve。后续 result 也因 pending 已删除而无法结束该 Promise。
- 影响：调用方若仍 await 返回 promise，取消后可能永久等待；静态控制流结论，未运行复现。
- 候选方向：定义一致的取消错误与终态，让显式取消只完成一次；并明确取消前未发送请求是否从发送队列移除。不能以发送了 cancel 就认为 JS 和 Native 两端均完成清理。
- 未来验收：取消前、执行中、完成后的取消及重复取消均有明确结果，无悬挂 Promise；用户决定待记录。

#### AR-010：WS 初始化失败后的未发送请求缺少有界失败处理

- 归属：R04；类别：缺陷；状态：已确认待决策；优先级：P2。
- 证据：`initialize_script.js::connectSocket` 在无 WebSocket 或构造抛错时直接返回；`hasLocalWebSocket` 表示存在凭据，不代表连接可用；streamCall 仍可加入 sendQueue，未见该队列的连接等待超时。
- 影响：在构造失败且未触发 close 清理等条件下，调用未到服务器，服务器超时不能解决等待。未宣称所有 CSP 失败都会表现为构造异常；具体平台触发需测试。
- 候选方向：传输就绪/失败状态与等待期限明确化；符合 D08 且请求尚未执行时才考虑 IPC，否则明确报错。已发送且结果未知的副作用请求不得自动重放。
- 未来验收：无 WebSocket、连接构造异常、握手拒绝、断线和恢复均有可解释终态，不无限排队；用户决定待记录。

R04 额外待核查：初始化脚本顶部存在无局部声明的 `EventListener = removeEventListener` 赋值。需在 R09/D03 整合时确认是否为误写全局及实际影响，当前不推断已破坏浏览器 API。

R04 Rust 侧补充证据：Luna 确认二进制为 18 字节头（版本、flags、BE u64 request ID、BE u64 seq）加消息体，解码拒绝短帧/错误版本。每调用 inbound 队列当前容量 64；默认服务器执行超时 30 秒（方法可覆盖），超时结果码 -2。取消信号与 handler future 竞速，不表示已经回滚副作用或强制停止底层阻塞操作。

#### AR-011：入站二进制队列满时静默丢块

- 归属：R04/R06/R08；类别：缺陷；状态：已确认待决策；优先级：P1（可能影响数据完整性）。
- 证据：`api_manager/mod.rs::on_binary` 对队列 try_send 失败只记录日志，没有终止调用或通知调用方；包含 END 的块也可能被丢弃。
- 影响：队列满时数据可能缺失；结束块丢失时消费者可能等待到超时或取消。静态条件性结论，未运行压力复现。
- 候选方向：明确流量控制与容量限制；无法接收时必须返回可识别失败并终止相关任务，不能让调用方误认为完整成功；具体策略待 R06 决定。
- 未来验收：慢消费者、队列饱和、接收端关闭和 END 边界都不发生静默数据损失。

#### AR-012：重复活跃请求 ID 可覆盖任务状态

- 归属：R04/R06；类别：协议健壮性；状态：已确认待决策；优先级：P2。
- 证据：`api_manager/mod.rs::dispatch` 将新 ActiveCall 插入三元组键，未见先拒绝重复活跃键；旧任务退出仍按同一键移除状态。
- 影响：异常客户端复用同连接 request ID 时，旧任务结束可能移除新任务状态。普通 JS 递增 ID 不代表服务器可以省略协议约束；未运行构造帧复现。
- 候选方向：明确连接内活跃 request ID 唯一，重复请求拒绝且不破坏原调用；未来验收覆盖重复 ID、旧任务结束及后续合法新调用。

上述 Rust 证据来自 `luna_deep_worker`（`/root/startup_boundaries`），主线程负责调用链整合和档案记录；本轮均未修改源码或执行测试。

#### R04 更正：os.info 按同步 XHR API 讲解与设计

日期：2026-09-25。用户指出 os.info 应为 XHR 同步 API。按此记录目标调用契约：JS 同步调用 → 本地 POST /__niva_sync → Rust 分发 → 同步返回结果/抛错，不再用它作为公开异步 WS API 的示例。

- 当前源码事实需与目标区分：`api/os.rs::register_apis` 将 os.info 注册为普通 unary handler；`api_manager/mod.rs::sync_method_allowed` 允许 os.* 进入同步路径。因此底层当前可由 WS call 或同步 XHR 触达，不能说已经限定为仅同步传输。此前实验中的 WS os.info 结果只说明传输可达，不决定未来公开 API 的同步契约；保留历史实测记录，不改写为同步实验。
- Rust handler 使用 async 函数并不决定页面 API 必须返回 Promise；页面的同步/异步契约和 Rust 内部实现方式是两个维度。
- 当前 Node 风格 OS 方法另有两类：arch/platform 等读取 bootstrap 元数据；cpus/freemem/networkInterfaces/uptime 使用 callSync。不能概括成所有 os 方法每次都发同步 XHR，也不能把 Node 标准 os 方法与自定义 os.info 混为一谈。
- D08 的“系统信息实现 fallback”须收窄解释：需要 Native 同步读取的 os.info 不以异步 IPC 冒充；没有同步通道时明确报错。已存在的启动快照/纯 JS 查询可保留同步返回；其他异步查询是否公开由 API 清单逐项决定，本轮不额外添加平行异步 API。
- R04 普通异步调用例子后续改用已确认异步契约的操作（例如目标接口中的异步文本文件读取），同时核对现行传输实现，不再借 os.info 混淆两条通道。

本轮只补充 review 档案，未修改 API 实现或类型。

### D09：三种 Bridge 的统一定义

日期：2026-09-25。来源：用户明确归纳。状态：已确认设计，作为 R04/R08 的通道与 API 分类依据；不代表全部实现已验收。

| Bridge | 页面调用契约 | 数据/任务范围 |
|---|---|---|
| WebSocket | 异步 | 一次性请求响应、流式任务、事件和二进制传输 |
| 同步 XHR | 同步 | 需要 Native 的同步 API，例如按用户确定契约的 os.info；同步返回或抛错 |
| 平台 IPC fallback | 一次性异步 | JSON 参数与结果，按 D08 实现可用的文件、网络等子集；拒绝流式、原始二进制传输和同步调用 |

- WS 的普通异步与流式是同一种 Bridge 的两类调用，不额外计为第四种 Bridge。
- IPC fallback 不改变公开 API 的同步/异步契约；异步接口仅在满足 JSON、权限和执行语义时可适配，不能将同步接口偷偷变成 Promise。
- 纯 JS 计算或读取已注入启动元数据不经过 Native Bridge，不计入三种通道。
- 三种通道均保持 Rust 侧鉴权和明确错误返回；传输降级不授予额外权限，也不自动重放结果未知的有副作用请求。
- 后续 API 清单分别记录：公开签名、同步/异步、支持通道、IPC fallback 可用范围、错误及取消契约。

本轮仅记录三种 Bridge 定义，没有修改代码。

### API 全量分类盘点 — 2026-09-25（本轮暂停 R05）

用户要求先列一张完整 API 分类表，按纯 JS、静态数据、WS、同步 XHR 和 IPC fallback 支持范围逐项 review，再继续后续章节。

本轮最新分类要求覆盖前面的 os.info 示例：**目标 os.info 是静态数据/启动快照读取，不是每次调用都过同步 XHR**。此前“os.info 按同步 XHR 设计”的段落保留为讨论历史，以本段和下表为准；动态 OS 信息仍按具体契约分类。当前源码 os.info 仍为可由 WS/XHR 调用的 Native handler，此差异列入表中，不声称静态入口已经落地。

统计边界：覆盖现有 Niva 顶层入口、Native 注册方法、当前 NodeCompat 模块公开导出与主要实例接口。别名（node:、require/ESM、未来 Niva 命名空间同一实现）不重复计数；同一通道的实例方法可在同一行完整列名。JavaScript/DOM/Web API 全集、第三方库私有函数不属于 Niva API 分母。旧 179 项清单仅用作覆盖检查，不采用其历史 missing/supported 状态来代替当前源码。

### 侧边讨论决定：crypto.timingSafeEqual 使用 JS 模拟

日期：2026-09-25。用户明确决定，状态：已决策待实现；本次只追加档案，不修改源码，不干扰主线程 API 盘点。

- 目标分类：纯 JS、同步返回，不需要 WS、同步 XHR 或 IPC fallback。替换当前调用 `os.timingSafeEqual` 的 Native 路径，不为这个方法维护专用 Native 实现；实施时核查移除专用依赖是否影响其他功能。
- 保留字节相等比较的基本契约：支持范围内的二进制输入按字节比较，不同长度报错；遍历全部字节并累积差异，不在首个不同字节处提前返回。
- 明确接受与 Node 安全语义的差异：JS 模拟不保证恒定时间或抵抗时序侧信道。不得将其标记为完整安全语义兼容，即使比较结果和常规功能测试通过。
- 用户要求调用时发出 warning，说明这是不保证时序安全的 JS 模拟。建议警告文本：`Niva: crypto.timingSafeEqual uses a JavaScript comparison and does not guarantee constant-time execution. Do not rely on it for timing-attack protection.` 警告不得包含入参、密钥、摘要或比较结果。
- 文档、公开类型说明及 API 分类表同步注明这一限制。当前源码仍为 Native + 同步 XHR；“纯 JS”是用户确定的目标，待整体 review 完成后实施。


#### API 分类总表（源码盘点 + review 目标）

> 实施阶段更新：下表保留review时源码快照，旧Niva.api/Native os.info等路径不代表新接口。用户现已授权执行方确定IPC范围；当前落地清单以实施台账和docs/bridge.md为准，最终仍需逐项验收。

**2026-09-25 用户确认留档：此表保留，后续还要逐项再次确认 IPC fallback 实现哪些。IPC 列中的“必做/建议/不做/待核”是当前讨论基线，不是最终冻结清单；三种 Bridge 的总体边界保留。不得在复核前将全部 IPC 行视为已最终批准实现。**

本表是单一总表，按层级区分入口。Node API 名称表示 `require(模块)`/ESM 对应出口；未来 `Niva.<模块>` 按 D01 共用同一实现。Native RPC 是当前 Rust 方法名，不代表未来还要对外保留一套同名旧 API。

**列的含义**：JS/静态、WS、XHR 以当前实现为准；仅明确标注“目标”的入口例外。IPC 列是 D08 的目标范围；Native 行另附当前 RPC 是否可达。`可达` 只说明通道接受该类 handler，不代表公开包装已实现 fallback、参数完全适配或任意来源已授权。所有 IPC 调用仍需窗口和精确来源授权，并受大小/超时等限制。

纯 JS 的 Promise、流对象或 Buffer 计算不需要 IPC；静态数据要先在获准上下文注入，不能理解成所有远端页面自动可读。同步字段/方法若实际查询 Native，不得降级成异步 Promise。一次性文件/HTTP fallback 也不能冒充 Node 流式契约。

表内共 **437 行/方法组**（含顶层辅助入口、事件/排除范围行、分组实例方法及 Native RPC；不是独立 Node API 数量）：页面入口 27 行、Node API 221 行、Native RPC 189 行。同类方法逐名列在同一格，别名不单独计功能。

| 层级 | API / 属性 / 明确分组 | JS / 静态数据 | WS | 同步 XHR | IPC fallback 目标 | 当前实现、限制与源码依据 |
|---|---|---|---|---|---|---|
| 页面入口 | `Niva.addEventListener` | JS/浏览器 | — | — | 无需 | 当前路径：JS；仅管理页面监听器；Native 事件来源另行分类，不保证 IPC 页面收到所有窗口事件 来源：`crates/niva/assets/initialize_script.js:56` |
| 页面入口 | `Niva.removeEventListener` | JS/浏览器 | — | — | 无需 | 当前路径：JS；仅管理页面监听器；Native 事件来源另行分类，不保证 IPC 页面收到所有窗口事件 来源：`crates/niva/assets/initialize_script.js:56` |
| 页面入口 | `Niva.removeAllEventListeners` | JS/浏览器 | — | — | 无需 | 当前路径：JS；仅管理页面监听器；Native 事件来源另行分类，不保证 IPC 页面收到所有窗口事件 来源：`crates/niva/assets/initialize_script.js:56` |
| 页面入口 | `Niva.registerModule` | JS/浏览器 | — | — | 无需 | 当前路径：JS；模块注册/查找；模块实际方法各自按表中路径调用 来源：`crates/niva/assets/initialize_script.js:616` |
| 页面入口 | `Niva.registerModuleFactory` | JS/浏览器 | — | — | 无需 | 当前路径：JS；模块注册/查找；模块实际方法各自按表中路径调用 来源：`crates/niva/assets/initialize_script.js:616` |
| 页面入口 | `Niva.require` | JS/浏览器 | — | — | 无需 | 当前路径：JS；模块注册/查找；模块实际方法各自按表中路径调用 来源：`crates/niva/assets/initialize_script.js:616` |
| 页面入口 | `Niva.import` | JS/浏览器 | — | — | 无需 | 当前路径：JS + 浏览器资源加载；注册表命中在 JS 完成；未命中可动态 import 资源 URL；不是 Native API 调用 来源：`crates/niva/assets/initialize_script.js:653` |
| 页面入口 | `Niva.bridgeVersion` | 静态快照/常量 | — | — | 无需 | 当前路径：静态；协议版本常量，不需要 Bridge 来源：`crates/niva/assets/initialize_script.js:65` |
| 页面入口 | `Niva.bootstrap.os / Niva.bootstrap.process` | 静态快照/常量 | — | — | 无需 | 当前路径：静态；获准上下文启动快照；process 仅主窗口顶层，不能因纯读取就扩大外源权限 来源：`crates/niva/src/app/node_bootstrap.rs:9` |
| 页面入口 | `Niva.os.info（用户确定的目标入口）` | 静态快照/常量 | — | — | 无需（须先授权注入） | 当前路径：静态；以本轮最新决定为准：启动注入数据，不每次调用Native。当前raw os.info方法仍可WS/XHR；两者不是同一层级的状态声明。 来源：`docs/architecture-review.md:API 全量分类盘点` |
| 页面入口 | `Niva.call` | — | 异步 | — | 部分 | 当前路径：WS / IPC；通用异步入口；具体方法须符合 JSON/授权/单次调用边界 来源：`crates/niva/assets/initialize_script.js:322` |
| 页面入口 | `Niva.callSync` | — | — | 是/按分支 | 否 | 当前路径：XHR；Native 同步入口；不因缺通道变为 Promise 来源：`crates/niva/assets/initialize_script.js:575` |
| 页面入口 | `Niva.stream / 返回句柄.cancel / 返回句柄.promise` | — | 流式 | — | 否 | 当前路径：WS流；任务句柄与取消逻辑在 JS，任务本身依赖 WS；现有取消缺口见 AR-009 来源：`crates/niva/assets/initialize_script.js:337` |
| 页面入口 | `Niva.streamSend` | — | 流式 | — | 否 | 当前路径：WS流；向任务发送二进制片段 来源：`crates/niva/assets/initialize_script.js:383` |
| 页面入口 | `Niva.api.<namespace>.<method>` | JS/浏览器 | 异步 | — | 按方法 | 当前路径：JS + WS / IPC；Proxy 只是调用包装，不说明任意名称存在；已注册的精确方法全部列于 Native 行 来源：`crates/niva/assets/initialize_script.js:440` |
| 页面入口 | `window.require / window.process / window.Buffer` | 静态快照/常量 | — | — | 按方法 | 当前路径：JS或静态；模块方法另列；兼容环境别名，不重复计模块 API；D01 要将注入开关与 Niva 命名空间分离 来源：`packages/node-compat/src/runtime/registration.js:101` |
| 页面入口 | `Niva.__emit__` | JS/浏览器 | — | — | 无需 | 当前路径：JS；内部事件分发入口，非独立 Native API；公开类型未承诺 来源：`crates/niva/assets/initialize_script.js:60` |
| 页面入口 | `原生事件：webview.loaded / webview.newWindowRequested / webview.downloadStarted / webview.permissionDenied / host:message / window.focused / window.scaleFactorChanged / window.themeChanged / window.closeRequested / window.message / menu.clicked / tray.rightClicked / tray.leftClicked / tray.doubleClicked / shortcut.emit / fileDrop.hovered / fileDrop.dropped / fileDrop.cancelled` | — | 事件 | — | 不属于一次性IPC；事件策略待定 | 当前路径：WS事件；这些是事件名而非可调用方法；addEventListener 是纯JS，不自动赋予远端事件接收能力 来源：`packages/types/Niva_zh.d.ts:424` |
| 页面入口 | `未提供的模块：perf_hooks / console(Node模块) / http2 / tty / v8 / vm / sqlite / test / cluster / worker_threads / readline；child_process.fork` | — | — | — | 否 | 当前路径：未实现/不在范围；浏览器 console/Worker 与 Node 对应模块不是同一契约；不计为已提供API 来源：`docs/node-api-inventory.json:excludedModules` |
| 页面入口 | `Niva.api.fs.read` | — | 流式 | — | 部分：文本必做，二进制不做 | 当前路径：WS流；当前仅本地WS启用JS override；IPC侧直发同名方法未注册。文本或Base64结果由流式读取聚合；Native无fs.read注册名 来源：`crates/niva/assets/initialize_script.js:509` |
| 页面入口 | `Niva.api.fs.write` | — | 流式 | — | 部分：文本必做，二进制不做 | 当前路径：WS流；当前仅本地WS启用JS override；IPC侧直发同名方法未注册。调用fs.writeStream，UTF-8或Base64解码后发送字节；Native无fs.write注册名 来源：`crates/niva/assets/initialize_script.js:509` |
| 页面入口 | `Niva.api.fs.append` | — | 流式 | — | 部分：文本必做，二进制不做 | 当前路径：WS流；当前仅本地WS启用JS override；IPC侧直发同名方法未注册。调用fs.writeStream追加模式；Native无fs.append注册名 来源：`crates/niva/assets/initialize_script.js:509` |
| 页面入口 | `Niva.api.process.exec` | — | 流式 | — | 建议：有界文本输出 | 当前路径：WS流；当前仅本地WS启用JS override；IPC侧直发同名方法未注册。调用process.execStream并聚合文本输出；Native无process.exec注册名 来源：`crates/niva/assets/initialize_script.js:509` |
| 页面入口 | `Niva.api.resource.read` | — | 流式 | — | 待定：文本资源候选，二进制不做 | 当前路径：WS流；当前仅本地WS启用JS override；IPC侧直发同名方法未注册。调用resource.readStream并聚合；Native无resource.read注册名 来源：`crates/niva/assets/initialize_script.js:509` |
| 页面入口 | `一次性 HTTP/HTTPS 文本/JSON请求（目标入口名待定）` | — | — | — | 必做 | 当前路径：待实现；非当前 Node http.request/get 流对象；返回状态码、响应头和有界文本/JSON。当前Native注册表尚无该一次HTTP handler。 来源：`docs/architecture-review.md:D08 实现范围表` |
| 页面入口 | `下载到文件 / 从文件上传（目标入口名待定）` | — | — | — | 建议实现 | 当前路径：待实现；Native执行传输，IPC只传路径/参数/最终结果；不跨IPC传二进制或进度流。 来源：`docs/architecture-review.md:D08 实现范围表` |
| 页面入口 | `有界子进程执行（目标兼容范围）` | — | — | — | 建议实现：文本输出 | 当前路径：待实现；沿用统一API；当前exec/execFile后端依赖WS流，需按有界结果契约适配，不支持交互stdin/实时输出。 来源：`docs/architecture-review.md:D08 实现范围表` |
| Node API | `fs.constants` | JS/浏览器 | — | — | 无需 | 当前路径：JS；常量 F_OK/R_OK/W_OK/X_OK、COPYFILE_EXCL/FICLONE/FICLONE_FORCE。 来源：`packages/node-compat/src/runtime/fs.js:7,96` |
| Node API | `fs.readFile` | — | 异步 | — | 必做（仅指定文本编码） | 当前路径：WS；异步路径走 fs.node unary；默认 Buffer 结果仍属二进制，IPC 必须拒绝；文本编码版本纳入 D08。 来源：`packages/node-compat/src/runtime/fs.js:69-75,85-110; packages/node-compat/src/runtime/registration.js:84-87` |
| Node API | `fs/promises.readFile` | — | 异步 | — | 必做（仅指定文本编码） | 当前路径：WS；异步路径走 fs.node unary；默认 Buffer 结果仍属二进制，IPC 必须拒绝；文本编码版本纳入 D08。 来源：`packages/node-compat/src/runtime/fs.js:69-75,85-110; packages/node-compat/src/runtime/registration.js:84-87` |
| Node API | `fs.writeFile` | — | 异步 | — | 必做（仅文本入参） | 当前路径：WS；异步字符串写/追加纳入 D08；Buffer、TypedArray 等二进制入参不经 IPC。 来源：`packages/node-compat/src/runtime/fs.js:31-45,85-110; packages/node-compat/src/runtime/registration.js:84-87` |
| Node API | `fs/promises.writeFile` | — | 异步 | — | 必做（仅文本入参） | 当前路径：WS；异步字符串写/追加纳入 D08；Buffer、TypedArray 等二进制入参不经 IPC。 来源：`packages/node-compat/src/runtime/fs.js:31-45,85-110; packages/node-compat/src/runtime/registration.js:84-87` |
| Node API | `fs.appendFile` | — | 异步 | — | 必做（仅文本入参） | 当前路径：WS；异步字符串写/追加纳入 D08；Buffer、TypedArray 等二进制入参不经 IPC。 来源：`packages/node-compat/src/runtime/fs.js:31-45,85-110; packages/node-compat/src/runtime/registration.js:84-87` |
| Node API | `fs/promises.appendFile` | — | 异步 | — | 必做（仅文本入参） | 当前路径：WS；异步字符串写/追加纳入 D08；Buffer、TypedArray 等二进制入参不经 IPC。 来源：`packages/node-compat/src/runtime/fs.js:31-45,85-110; packages/node-compat/src/runtime/registration.js:84-87` |
| Node API | `fs.mkdir` | — | 异步 | — | 必做 | 当前路径：WS；一次性异步文件/目录 JSON 操作；D08 包含目录、元数据、创建/重命名/删除与复制。cp 当前为 JS 递归编排，IPC 目标需 Native 一次性复制。readdir 的 buffer 编码仍属二进制边界。 来源：`packages/node-compat/src/runtime/fs.js:31-84,96-142; docs/node-api-inventory.json D08实现范围表 550-569` |
| Node API | `fs/promises.mkdir` | — | 异步 | — | 必做 | 当前路径：WS；一次性异步文件/目录 JSON 操作；D08 包含目录、元数据、创建/重命名/删除与复制。cp 当前为 JS 递归编排，IPC 目标需 Native 一次性复制。readdir 的 buffer 编码仍属二进制边界。 来源：`packages/node-compat/src/runtime/fs.js:31-84,96-142; docs/node-api-inventory.json D08实现范围表 550-569` |
| Node API | `fs.readdir` | — | 异步 | — | 必做 | 当前路径：WS；一次性异步文件/目录 JSON 操作；D08 包含目录、元数据、创建/重命名/删除与复制。cp 当前为 JS 递归编排，IPC 目标需 Native 一次性复制。readdir 的 buffer 编码仍属二进制边界。 来源：`packages/node-compat/src/runtime/fs.js:31-84,96-142; docs/node-api-inventory.json D08实现范围表 550-569` |
| Node API | `fs/promises.readdir` | — | 异步 | — | 必做 | 当前路径：WS；一次性异步文件/目录 JSON 操作；D08 包含目录、元数据、创建/重命名/删除与复制。cp 当前为 JS 递归编排，IPC 目标需 Native 一次性复制。readdir 的 buffer 编码仍属二进制边界。 来源：`packages/node-compat/src/runtime/fs.js:31-84,96-142; docs/node-api-inventory.json D08实现范围表 550-569` |
| Node API | `fs.stat` | — | 异步 | — | 必做 | 当前路径：WS；一次性异步文件/目录 JSON 操作；D08 包含目录、元数据、创建/重命名/删除与复制。cp 当前为 JS 递归编排，IPC 目标需 Native 一次性复制。readdir 的 buffer 编码仍属二进制边界。 来源：`packages/node-compat/src/runtime/fs.js:31-84,96-142; docs/node-api-inventory.json D08实现范围表 550-569` |
| Node API | `fs/promises.stat` | — | 异步 | — | 必做 | 当前路径：WS；一次性异步文件/目录 JSON 操作；D08 包含目录、元数据、创建/重命名/删除与复制。cp 当前为 JS 递归编排，IPC 目标需 Native 一次性复制。readdir 的 buffer 编码仍属二进制边界。 来源：`packages/node-compat/src/runtime/fs.js:31-84,96-142; docs/node-api-inventory.json D08实现范围表 550-569` |
| Node API | `fs.lstat` | — | 异步 | — | 必做 | 当前路径：WS；一次性异步文件/目录 JSON 操作；D08 包含目录、元数据、创建/重命名/删除与复制。cp 当前为 JS 递归编排，IPC 目标需 Native 一次性复制。readdir 的 buffer 编码仍属二进制边界。 来源：`packages/node-compat/src/runtime/fs.js:31-84,96-142; docs/node-api-inventory.json D08实现范围表 550-569` |
| Node API | `fs/promises.lstat` | — | 异步 | — | 必做 | 当前路径：WS；一次性异步文件/目录 JSON 操作；D08 包含目录、元数据、创建/重命名/删除与复制。cp 当前为 JS 递归编排，IPC 目标需 Native 一次性复制。readdir 的 buffer 编码仍属二进制边界。 来源：`packages/node-compat/src/runtime/fs.js:31-84,96-142; docs/node-api-inventory.json D08实现范围表 550-569` |
| Node API | `fs.realpath` | — | 异步 | — | 必做 | 当前路径：WS；一次性异步文件/目录 JSON 操作；D08 包含目录、元数据、创建/重命名/删除与复制。cp 当前为 JS 递归编排，IPC 目标需 Native 一次性复制。readdir 的 buffer 编码仍属二进制边界。 来源：`packages/node-compat/src/runtime/fs.js:31-84,96-142; docs/node-api-inventory.json D08实现范围表 550-569` |
| Node API | `fs/promises.realpath` | — | 异步 | — | 必做 | 当前路径：WS；一次性异步文件/目录 JSON 操作；D08 包含目录、元数据、创建/重命名/删除与复制。cp 当前为 JS 递归编排，IPC 目标需 Native 一次性复制。readdir 的 buffer 编码仍属二进制边界。 来源：`packages/node-compat/src/runtime/fs.js:31-84,96-142; docs/node-api-inventory.json D08实现范围表 550-569` |
| Node API | `fs.rename` | — | 异步 | — | 必做 | 当前路径：WS；一次性异步文件/目录 JSON 操作；D08 包含目录、元数据、创建/重命名/删除与复制。cp 当前为 JS 递归编排，IPC 目标需 Native 一次性复制。readdir 的 buffer 编码仍属二进制边界。 来源：`packages/node-compat/src/runtime/fs.js:31-84,96-142; docs/node-api-inventory.json D08实现范围表 550-569` |
| Node API | `fs/promises.rename` | — | 异步 | — | 必做 | 当前路径：WS；一次性异步文件/目录 JSON 操作；D08 包含目录、元数据、创建/重命名/删除与复制。cp 当前为 JS 递归编排，IPC 目标需 Native 一次性复制。readdir 的 buffer 编码仍属二进制边界。 来源：`packages/node-compat/src/runtime/fs.js:31-84,96-142; docs/node-api-inventory.json D08实现范围表 550-569` |
| Node API | `fs.access` | — | 异步 | — | 必做 | 当前路径：WS；一次性异步文件/目录 JSON 操作；D08 包含目录、元数据、创建/重命名/删除与复制。cp 当前为 JS 递归编排，IPC 目标需 Native 一次性复制。readdir 的 buffer 编码仍属二进制边界。 来源：`packages/node-compat/src/runtime/fs.js:31-84,96-142; docs/node-api-inventory.json D08实现范围表 550-569` |
| Node API | `fs/promises.access` | — | 异步 | — | 必做 | 当前路径：WS；一次性异步文件/目录 JSON 操作；D08 包含目录、元数据、创建/重命名/删除与复制。cp 当前为 JS 递归编排，IPC 目标需 Native 一次性复制。readdir 的 buffer 编码仍属二进制边界。 来源：`packages/node-compat/src/runtime/fs.js:31-84,96-142; docs/node-api-inventory.json D08实现范围表 550-569` |
| Node API | `fs.rm` | — | 异步 | — | 必做 | 当前路径：WS；一次性异步文件/目录 JSON 操作；D08 包含目录、元数据、创建/重命名/删除与复制。cp 当前为 JS 递归编排，IPC 目标需 Native 一次性复制。readdir 的 buffer 编码仍属二进制边界。 来源：`packages/node-compat/src/runtime/fs.js:31-84,96-142; docs/node-api-inventory.json D08实现范围表 550-569` |
| Node API | `fs/promises.rm` | — | 异步 | — | 必做 | 当前路径：WS；一次性异步文件/目录 JSON 操作；D08 包含目录、元数据、创建/重命名/删除与复制。cp 当前为 JS 递归编排，IPC 目标需 Native 一次性复制。readdir 的 buffer 编码仍属二进制边界。 来源：`packages/node-compat/src/runtime/fs.js:31-84,96-142; docs/node-api-inventory.json D08实现范围表 550-569` |
| Node API | `fs.unlink` | — | 异步 | — | 必做 | 当前路径：WS；一次性异步文件/目录 JSON 操作；D08 包含目录、元数据、创建/重命名/删除与复制。cp 当前为 JS 递归编排，IPC 目标需 Native 一次性复制。readdir 的 buffer 编码仍属二进制边界。 来源：`packages/node-compat/src/runtime/fs.js:31-84,96-142; docs/node-api-inventory.json D08实现范围表 550-569` |
| Node API | `fs/promises.unlink` | — | 异步 | — | 必做 | 当前路径：WS；一次性异步文件/目录 JSON 操作；D08 包含目录、元数据、创建/重命名/删除与复制。cp 当前为 JS 递归编排，IPC 目标需 Native 一次性复制。readdir 的 buffer 编码仍属二进制边界。 来源：`packages/node-compat/src/runtime/fs.js:31-84,96-142; docs/node-api-inventory.json D08实现范围表 550-569` |
| Node API | `fs.copyFile` | — | 异步 | — | 必做 | 当前路径：WS；一次性异步文件/目录 JSON 操作；D08 包含目录、元数据、创建/重命名/删除与复制。cp 当前为 JS 递归编排，IPC 目标需 Native 一次性复制。readdir 的 buffer 编码仍属二进制边界。 来源：`packages/node-compat/src/runtime/fs.js:31-84,96-142; docs/node-api-inventory.json D08实现范围表 550-569` |
| Node API | `fs/promises.copyFile` | — | 异步 | — | 必做 | 当前路径：WS；一次性异步文件/目录 JSON 操作；D08 包含目录、元数据、创建/重命名/删除与复制。cp 当前为 JS 递归编排，IPC 目标需 Native 一次性复制。readdir 的 buffer 编码仍属二进制边界。 来源：`packages/node-compat/src/runtime/fs.js:31-84,96-142; docs/node-api-inventory.json D08实现范围表 550-569` |
| Node API | `fs.readFileSync` | — | — | 是/按分支 | 不做（同步调用） | 当前路径：XHR；每项均走 callSync；跨 Native 同步 IPC 不在 D08 范围。existsSync 以 accessSync 包装。 来源：`packages/node-compat/src/runtime/fs.js:85-95,97-120; packages/node-compat/src/fs.js:25-38` |
| Node API | `fs.writeFileSync` | — | — | 是/按分支 | 不做（同步调用） | 当前路径：XHR；每项均走 callSync；跨 Native 同步 IPC 不在 D08 范围。existsSync 以 accessSync 包装。 来源：`packages/node-compat/src/runtime/fs.js:85-95,97-120; packages/node-compat/src/fs.js:25-38` |
| Node API | `fs.appendFileSync` | — | — | 是/按分支 | 不做（同步调用） | 当前路径：XHR；每项均走 callSync；跨 Native 同步 IPC 不在 D08 范围。existsSync 以 accessSync 包装。 来源：`packages/node-compat/src/runtime/fs.js:85-95,97-120; packages/node-compat/src/fs.js:25-38` |
| Node API | `fs.mkdirSync` | — | — | 是/按分支 | 不做（同步调用） | 当前路径：XHR；每项均走 callSync；跨 Native 同步 IPC 不在 D08 范围。existsSync 以 accessSync 包装。 来源：`packages/node-compat/src/runtime/fs.js:85-95,97-120; packages/node-compat/src/fs.js:25-38` |
| Node API | `fs.readdirSync` | — | — | 是/按分支 | 不做（同步调用） | 当前路径：XHR；每项均走 callSync；跨 Native 同步 IPC 不在 D08 范围。existsSync 以 accessSync 包装。 来源：`packages/node-compat/src/runtime/fs.js:85-95,97-120; packages/node-compat/src/fs.js:25-38` |
| Node API | `fs.statSync` | — | — | 是/按分支 | 不做（同步调用） | 当前路径：XHR；每项均走 callSync；跨 Native 同步 IPC 不在 D08 范围。existsSync 以 accessSync 包装。 来源：`packages/node-compat/src/runtime/fs.js:85-95,97-120; packages/node-compat/src/fs.js:25-38` |
| Node API | `fs.lstatSync` | — | — | 是/按分支 | 不做（同步调用） | 当前路径：XHR；每项均走 callSync；跨 Native 同步 IPC 不在 D08 范围。existsSync 以 accessSync 包装。 来源：`packages/node-compat/src/runtime/fs.js:85-95,97-120; packages/node-compat/src/fs.js:25-38` |
| Node API | `fs.realpathSync` | — | — | 是/按分支 | 不做（同步调用） | 当前路径：XHR；每项均走 callSync；跨 Native 同步 IPC 不在 D08 范围。existsSync 以 accessSync 包装。 来源：`packages/node-compat/src/runtime/fs.js:85-95,97-120; packages/node-compat/src/fs.js:25-38` |
| Node API | `fs.renameSync` | — | — | 是/按分支 | 不做（同步调用） | 当前路径：XHR；每项均走 callSync；跨 Native 同步 IPC 不在 D08 范围。existsSync 以 accessSync 包装。 来源：`packages/node-compat/src/runtime/fs.js:85-95,97-120; packages/node-compat/src/fs.js:25-38` |
| Node API | `fs.copyFileSync` | — | — | 是/按分支 | 不做（同步调用） | 当前路径：XHR；每项均走 callSync；跨 Native 同步 IPC 不在 D08 范围。existsSync 以 accessSync 包装。 来源：`packages/node-compat/src/runtime/fs.js:85-95,97-120; packages/node-compat/src/fs.js:25-38` |
| Node API | `fs.accessSync` | — | — | 是/按分支 | 不做（同步调用） | 当前路径：XHR；每项均走 callSync；跨 Native 同步 IPC 不在 D08 范围。existsSync 以 accessSync 包装。 来源：`packages/node-compat/src/runtime/fs.js:85-95,97-120; packages/node-compat/src/fs.js:25-38` |
| Node API | `fs.rmSync` | — | — | 是/按分支 | 不做（同步调用） | 当前路径：XHR；每项均走 callSync；跨 Native 同步 IPC 不在 D08 范围。existsSync 以 accessSync 包装。 来源：`packages/node-compat/src/runtime/fs.js:85-95,97-120; packages/node-compat/src/fs.js:25-38` |
| Node API | `fs.unlinkSync` | — | — | 是/按分支 | 不做（同步调用） | 当前路径：XHR；每项均走 callSync；跨 Native 同步 IPC 不在 D08 范围。existsSync 以 accessSync 包装。 来源：`packages/node-compat/src/runtime/fs.js:85-95,97-120; packages/node-compat/src/fs.js:25-38` |
| Node API | `fs.existsSync` | — | — | 是/按分支 | 不做（同步调用） | 当前路径：XHR；每项均走 callSync；跨 Native 同步 IPC 不在 D08 范围。existsSync 以 accessSync 包装。 来源：`packages/node-compat/src/runtime/fs.js:85-95,97-120; packages/node-compat/src/fs.js:25-38` |
| Node API | `fs.createReadStream` | — | 流式 | — | 不做（流/句柄） | 当前路径：WS流；流通过 FileHandle 的 open/read/write/close 操作保持 Node Readable/Writable 或句柄契约；D08 不做文件流、句柄及二进制跨 IPC。 来源：`packages/node-compat/src/runtime/fs.js:143-255` |
| Node API | `fs.createWriteStream` | — | 流式 | — | 不做（流/句柄） | 当前路径：WS流；流通过 FileHandle 的 open/read/write/close 操作保持 Node Readable/Writable 或句柄契约；D08 不做文件流、句柄及二进制跨 IPC。 来源：`packages/node-compat/src/runtime/fs.js:143-255` |
| Node API | `fs/promises.open` | — | 流式 | — | 不做（流/句柄） | 当前路径：WS流；流通过 FileHandle 的 open/read/write/close 操作保持 Node Readable/Writable 或句柄契约；D08 不做文件流、句柄及二进制跨 IPC。 来源：`packages/node-compat/src/runtime/fs.js:143-255` |
| Node API | `fs.watch` | — | 流式 | — | 不做（持续订阅） | 当前路径：WS流；watcher 提供 close/ref/unref 与 change/error/close 事件；持续监听不作为一次性 IPC。 来源：`packages/node-compat/src/runtime/fs.js:257-283` |
| Node API | `fs.FileHandle` | — | 流式 | — | 不做（句柄/二进制） | 当前路径：WS流；open 返回普通对象而非导出类；fd；close/stat/sync/datasync/truncate/read/write/readFile/writeFile/appendFile。read/write 接受 Buffer/视图并回传字节；D08 不做。 来源：`packages/node-compat/src/runtime/fs.js:151-220` |
| Node API | `fs.Stats / fs.Dirent-like result` | JS/浏览器 | — | — | 必做（元数据 JSON） | 当前路径：JS；结果对象及 is* 判断在JS；原始元数据由stat/lstat/readdir经WS或XHR取得，见对应行。无 Stats/Dirent 构造器；stat/lstat 与 readdir(withFileTypes) 生成 plain object。Stats 字段 dev/ino/mode/nlink/uid/gid/rdev/size/blksize/blocks、四个 *Ms 和四个 Date；isFile/isDirectory/isSymbolicLink/isBlockDevice/isCharacterDevice/isFIFO/isSocket。Dirent-like 另有 name/parentPath/path。IPC 回包需 JS 补回 is* 方法。 来源：`packages/node-compat/src/runtime/fs.js:20-29,69-80` |
| Node API | `fs.ReadStream / fs.WriteStream instances` | — | 流式 | — | 不做（文件流） | 当前路径：WS流；分别是 vendor Readable/Writable；暴露 Node stream 的读/写、销毁和事件接口，并由 FileHandle 传输。 来源：`packages/node-compat/src/runtime/fs.js:222-255` |
| Node API | `process.arch` | 静态快照/常量 | — | — | 无需（启动快照） | 当前路径：静态；值来自 Niva bootstrap.process；env 和 argv 在 JS 侧复制。 来源：`crates/niva/src/app/node_bootstrap.rs:66-79; packages/node-compat/src/runtime/process.js:5-11; packages/node-compat/src/process.js:5-9` |
| Node API | `process.argv` | 静态快照/常量 | — | — | 无需（启动快照） | 当前路径：静态；值来自 Niva bootstrap.process；env 和 argv 在 JS 侧复制。 来源：`crates/niva/src/app/node_bootstrap.rs:66-79; packages/node-compat/src/runtime/process.js:5-11; packages/node-compat/src/process.js:5-9` |
| Node API | `process.argv0` | 静态快照/常量 | — | — | 无需（启动快照） | 当前路径：静态；值来自 Niva bootstrap.process；env 和 argv 在 JS 侧复制。 来源：`crates/niva/src/app/node_bootstrap.rs:66-79; packages/node-compat/src/runtime/process.js:5-11; packages/node-compat/src/process.js:5-9` |
| Node API | `process.env` | 静态快照/常量 | — | — | 无需（启动快照） | 当前路径：静态；值来自 Niva bootstrap.process；env 和 argv 在 JS 侧复制。 来源：`crates/niva/src/app/node_bootstrap.rs:66-79; packages/node-compat/src/runtime/process.js:5-11; packages/node-compat/src/process.js:5-9` |
| Node API | `process.execPath` | 静态快照/常量 | — | — | 无需（启动快照） | 当前路径：静态；值来自 Niva bootstrap.process；env 和 argv 在 JS 侧复制。 来源：`crates/niva/src/app/node_bootstrap.rs:66-79; packages/node-compat/src/runtime/process.js:5-11; packages/node-compat/src/process.js:5-9` |
| Node API | `process.pid` | 静态快照/常量 | — | — | 无需（启动快照） | 当前路径：静态；值来自 Niva bootstrap.process；env 和 argv 在 JS 侧复制。 来源：`crates/niva/src/app/node_bootstrap.rs:66-79; packages/node-compat/src/runtime/process.js:5-11; packages/node-compat/src/process.js:5-9` |
| Node API | `process.platform` | 静态快照/常量 | — | — | 无需（启动快照） | 当前路径：静态；值来自 Niva bootstrap.process；env 和 argv 在 JS 侧复制。 来源：`crates/niva/src/app/node_bootstrap.rs:66-79; packages/node-compat/src/runtime/process.js:5-11; packages/node-compat/src/process.js:5-9` |
| Node API | `process.version` | 静态快照/常量 | — | — | 无需（启动快照） | 当前路径：静态；值来自 Niva bootstrap.process；env 和 argv 在 JS 侧复制。 来源：`crates/niva/src/app/node_bootstrap.rs:66-79; packages/node-compat/src/runtime/process.js:5-11; packages/node-compat/src/process.js:5-9` |
| Node API | `process.versions` | 静态快照/常量 | — | — | 无需（启动快照） | 当前路径：静态；值来自 Niva bootstrap.process；env 和 argv 在 JS 侧复制。 来源：`crates/niva/src/app/node_bootstrap.rs:66-79; packages/node-compat/src/runtime/process.js:5-11; packages/node-compat/src/process.js:5-9` |
| Node API | `process.cwd` | — | — | 是/按分支 | 不做（同步 Native 调用） | 当前路径：XHR；分别 callSync process.currentDir / process.setCurrentDir；不改成异步 IPC。 来源：`packages/node-compat/src/runtime/process.js:12-13` |
| Node API | `process.chdir` | — | — | 是/按分支 | 不做（同步 Native 调用） | 当前路径：XHR；分别 callSync process.currentDir / process.setCurrentDir；不改成异步 IPC。 来源：`packages/node-compat/src/runtime/process.js:12-13` |
| Node API | `process.nextTick` | JS/浏览器 | — | — | 无需 | 当前路径：JS；queueMicrotask 本地调度。 来源：`packages/node-compat/src/runtime/process.js:14-15` |
| Node API | `process.exit` | — | 异步 | — | 待核 | 当前路径：WS；先 emit(exit)，再 call process.exit；一次性 WS 调用。D08 未为终止进程单独定 IPC 子集。 来源：`packages/node-compat/src/runtime/process.js:16-17` |
| Node API | `process.stdout.write` | — | 异步 | — | 建议（文本有界） | 当前路径：WS；Writable 每个 write 调用 process.write，当前按 chunk unary WS 发送 base64；D08 仅建议有限文本输出，二进制拒绝。 来源：`packages/node-compat/src/runtime/process.js:18-20` |
| Node API | `process.stderr.write` | — | 异步 | — | 建议（文本有界） | 当前路径：WS；Writable 每个 write 调用 process.write，当前按 chunk unary WS 发送 base64；D08 仅建议有限文本输出，二进制拒绝。 来源：`packages/node-compat/src/runtime/process.js:18-20` |
| Node API | `process.stdin` | — | 流式 | — | 不做（交互流） | 当前路径：WS流；Readable 通过 process.stdin 持续接收二进制 chunk。 来源：`packages/node-compat/src/runtime/process.js:21-24` |
| Node API | `process.on` | JS/浏览器 | — | — | 无需 | 当前路径：JS；Named export；默认 process 还继承 EventEmitter 的 addListener/once/off/removeListener/removeAllListeners/listeners/listenerCount/emit。exitCode 是本地 JS 属性。 来源：`packages/node-compat/src/runtime/process.js:10-17; packages/node-compat/src/runtime/bridge.js:73-118; packages/node-compat/src/process.js:7-9` |
| Node API | `process.EventEmitter/exitCode surface` | JS/浏览器 | — | — | 无需 | 当前路径：JS；默认导出对象的本地事件方法 addListener/once/off/removeListener/removeAllListeners/listeners/listenerCount/emit；exitCode。 来源：`packages/node-compat/src/runtime/process.js:10-17; packages/node-compat/src/runtime/bridge.js:73-118` |
| Node API | `os.EOL` | 静态快照/常量 | — | — | 无需（启动快照） | 当前路径：静态；从 bootstrap.os 读取；userInfo 按 encoding 可在 JS 包成 Buffer。devNull 也是 default os 对象静态 getter，但未 named-export。 来源：`crates/niva/src/app/node_bootstrap.rs:46-64; packages/node-compat/src/runtime/os.js:5-19; packages/node-compat/src/os.js:5-8` |
| Node API | `os.arch` | 静态快照/常量 | — | — | 无需（启动快照） | 当前路径：静态；从 bootstrap.os 读取；userInfo 按 encoding 可在 JS 包成 Buffer。devNull 也是 default os 对象静态 getter，但未 named-export。 来源：`crates/niva/src/app/node_bootstrap.rs:46-64; packages/node-compat/src/runtime/os.js:5-19; packages/node-compat/src/os.js:5-8` |
| Node API | `os.platform` | 静态快照/常量 | — | — | 无需（启动快照） | 当前路径：静态；从 bootstrap.os 读取；userInfo 按 encoding 可在 JS 包成 Buffer。devNull 也是 default os 对象静态 getter，但未 named-export。 来源：`crates/niva/src/app/node_bootstrap.rs:46-64; packages/node-compat/src/runtime/os.js:5-19; packages/node-compat/src/os.js:5-8` |
| Node API | `os.homedir` | 静态快照/常量 | — | — | 无需（启动快照） | 当前路径：静态；从 bootstrap.os 读取；userInfo 按 encoding 可在 JS 包成 Buffer。devNull 也是 default os 对象静态 getter，但未 named-export。 来源：`crates/niva/src/app/node_bootstrap.rs:46-64; packages/node-compat/src/runtime/os.js:5-19; packages/node-compat/src/os.js:5-8` |
| Node API | `os.tmpdir` | 静态快照/常量 | — | — | 无需（启动快照） | 当前路径：静态；从 bootstrap.os 读取；userInfo 按 encoding 可在 JS 包成 Buffer。devNull 也是 default os 对象静态 getter，但未 named-export。 来源：`crates/niva/src/app/node_bootstrap.rs:46-64; packages/node-compat/src/runtime/os.js:5-19; packages/node-compat/src/os.js:5-8` |
| Node API | `os.hostname` | 静态快照/常量 | — | — | 无需（启动快照） | 当前路径：静态；从 bootstrap.os 读取；userInfo 按 encoding 可在 JS 包成 Buffer。devNull 也是 default os 对象静态 getter，但未 named-export。 来源：`crates/niva/src/app/node_bootstrap.rs:46-64; packages/node-compat/src/runtime/os.js:5-19; packages/node-compat/src/os.js:5-8` |
| Node API | `os.release` | 静态快照/常量 | — | — | 无需（启动快照） | 当前路径：静态；从 bootstrap.os 读取；userInfo 按 encoding 可在 JS 包成 Buffer。devNull 也是 default os 对象静态 getter，但未 named-export。 来源：`crates/niva/src/app/node_bootstrap.rs:46-64; packages/node-compat/src/runtime/os.js:5-19; packages/node-compat/src/os.js:5-8` |
| Node API | `os.totalmem` | 静态快照/常量 | — | — | 无需（启动快照） | 当前路径：静态；从 bootstrap.os 读取；userInfo 按 encoding 可在 JS 包成 Buffer。devNull 也是 default os 对象静态 getter，但未 named-export。 来源：`crates/niva/src/app/node_bootstrap.rs:46-64; packages/node-compat/src/runtime/os.js:5-19; packages/node-compat/src/os.js:5-8` |
| Node API | `os.type` | 静态快照/常量 | — | — | 无需（启动快照） | 当前路径：静态；从 bootstrap.os 读取；userInfo 按 encoding 可在 JS 包成 Buffer。devNull 也是 default os 对象静态 getter，但未 named-export。 来源：`crates/niva/src/app/node_bootstrap.rs:46-64; packages/node-compat/src/runtime/os.js:5-19; packages/node-compat/src/os.js:5-8` |
| Node API | `os.version` | 静态快照/常量 | — | — | 无需（启动快照） | 当前路径：静态；从 bootstrap.os 读取；userInfo 按 encoding 可在 JS 包成 Buffer。devNull 也是 default os 对象静态 getter，但未 named-export。 来源：`crates/niva/src/app/node_bootstrap.rs:46-64; packages/node-compat/src/runtime/os.js:5-19; packages/node-compat/src/os.js:5-8` |
| Node API | `os.userInfo` | 静态快照/常量 | — | — | 无需（启动快照） | 当前路径：静态；从 bootstrap.os 读取；userInfo 按 encoding 可在 JS 包成 Buffer。devNull 也是 default os 对象静态 getter，但未 named-export。 来源：`crates/niva/src/app/node_bootstrap.rs:46-64; packages/node-compat/src/runtime/os.js:5-19; packages/node-compat/src/os.js:5-8` |
| Node API | `os.cpus` | — | — | 是/按分支 | 不做（Node 方法同步） | 当前路径：XHR；动态值通过 callSync os.* 获取；D08 IPC 为异步单次调用，不能把标准同步 Node 方法改成 Promise。 来源：`packages/node-compat/src/runtime/os.js:11; packages/node-compat/src/os.js:7` |
| Node API | `os.freemem` | — | — | 是/按分支 | 不做（Node 方法同步） | 当前路径：XHR；动态值通过 callSync os.* 获取；D08 IPC 为异步单次调用，不能把标准同步 Node 方法改成 Promise。 来源：`packages/node-compat/src/runtime/os.js:11; packages/node-compat/src/os.js:7` |
| Node API | `os.networkInterfaces` | — | — | 是/按分支 | 不做（Node 方法同步） | 当前路径：XHR；动态值通过 callSync os.* 获取；D08 IPC 为异步单次调用，不能把标准同步 Node 方法改成 Promise。 来源：`packages/node-compat/src/runtime/os.js:11; packages/node-compat/src/os.js:7` |
| Node API | `os.uptime` | — | — | 是/按分支 | 不做（Node 方法同步） | 当前路径：XHR；动态值通过 callSync os.* 获取；D08 IPC 为异步单次调用，不能把标准同步 Node 方法改成 Promise。 来源：`packages/node-compat/src/runtime/os.js:11; packages/node-compat/src/os.js:7` |
| Node API | `os.devNull` | 静态快照/常量 | — | — | 无需 | 当前路径：静态；default os 对象的 getter，按 bootstrap.os.platform 返回平台路径；没有单独 named export。 来源：`packages/node-compat/src/runtime/os.js:16-19` |
| Node API | `path.basename` | JS/浏览器 | — | — | 无需 | 当前路径：JS；纯 JS 路径算法/常量；path.posix 与 path.win32 子对象均提供同一套方法。 来源：`packages/node-compat/src/runtime/path.js:185-205,306-353,361-490,558-727; packages/node-compat/src/path.js:6-21` |
| Node API | `path.delimiter` | JS/浏览器 | — | — | 无需 | 当前路径：JS；纯 JS 路径算法/常量；path.posix 与 path.win32 子对象均提供同一套方法。 来源：`packages/node-compat/src/runtime/path.js:185-205,306-353,361-490,558-727; packages/node-compat/src/path.js:6-21` |
| Node API | `path.dirname` | JS/浏览器 | — | — | 无需 | 当前路径：JS；纯 JS 路径算法/常量；path.posix 与 path.win32 子对象均提供同一套方法。 来源：`packages/node-compat/src/runtime/path.js:185-205,306-353,361-490,558-727; packages/node-compat/src/path.js:6-21` |
| Node API | `path.extname` | JS/浏览器 | — | — | 无需 | 当前路径：JS；纯 JS 路径算法/常量；path.posix 与 path.win32 子对象均提供同一套方法。 来源：`packages/node-compat/src/runtime/path.js:185-205,306-353,361-490,558-727; packages/node-compat/src/path.js:6-21` |
| Node API | `path.format` | JS/浏览器 | — | — | 无需 | 当前路径：JS；纯 JS 路径算法/常量；path.posix 与 path.win32 子对象均提供同一套方法。 来源：`packages/node-compat/src/runtime/path.js:185-205,306-353,361-490,558-727; packages/node-compat/src/path.js:6-21` |
| Node API | `path.isAbsolute` | JS/浏览器 | — | — | 无需 | 当前路径：JS；纯 JS 路径算法/常量；path.posix 与 path.win32 子对象均提供同一套方法。 来源：`packages/node-compat/src/runtime/path.js:185-205,306-353,361-490,558-727; packages/node-compat/src/path.js:6-21` |
| Node API | `path.join` | JS/浏览器 | — | — | 无需 | 当前路径：JS；纯 JS 路径算法/常量；path.posix 与 path.win32 子对象均提供同一套方法。 来源：`packages/node-compat/src/runtime/path.js:185-205,306-353,361-490,558-727; packages/node-compat/src/path.js:6-21` |
| Node API | `path.normalize` | JS/浏览器 | — | — | 无需 | 当前路径：JS；纯 JS 路径算法/常量；path.posix 与 path.win32 子对象均提供同一套方法。 来源：`packages/node-compat/src/runtime/path.js:185-205,306-353,361-490,558-727; packages/node-compat/src/path.js:6-21` |
| Node API | `path.parse` | JS/浏览器 | — | — | 无需 | 当前路径：JS；纯 JS 路径算法/常量；path.posix 与 path.win32 子对象均提供同一套方法。 来源：`packages/node-compat/src/runtime/path.js:185-205,306-353,361-490,558-727; packages/node-compat/src/path.js:6-21` |
| Node API | `path.sep` | JS/浏览器 | — | — | 无需 | 当前路径：JS；纯 JS 路径算法/常量；path.posix 与 path.win32 子对象均提供同一套方法。 来源：`packages/node-compat/src/runtime/path.js:185-205,306-353,361-490,558-727; packages/node-compat/src/path.js:6-21` |
| Node API | `path.matchesGlob` | JS/浏览器 | — | — | 无需 | 当前路径：JS；纯 JS 路径算法/常量；path.posix 与 path.win32 子对象均提供同一套方法。 来源：`packages/node-compat/src/runtime/path.js:185-205,306-353,361-490,558-727; packages/node-compat/src/path.js:6-21` |
| Node API | `path.resolve` | JS/浏览器 | — | 是/按分支 | 不做（同步 cwd 查询） | 当前路径：混合(JS+XHR)；resolve 每次取 currentCwd，relative 通过两次 resolve；默认经 process.cwd/callSync process.currentDir。setCwd 覆盖后纯 JS。 来源：`packages/node-compat/src/runtime/path.js:45-60,252-304,510-545,737-743` |
| Node API | `path.relative` | JS/浏览器 | — | 是/按分支 | 不做（同步 cwd 查询） | 当前路径：混合(JS+XHR)；resolve 每次取 currentCwd，relative 通过两次 resolve；默认经 process.cwd/callSync process.currentDir。setCwd 覆盖后纯 JS。 来源：`packages/node-compat/src/runtime/path.js:45-60,252-304,510-545,737-743` |
| Node API | `path.getCwd` | JS/浏览器 | — | 是/按分支 | 不做（同步 cwd 查询） | 当前路径：混合(JS+XHR)；resolve 每次取 currentCwd，relative 通过两次 resolve；默认经 process.cwd/callSync process.currentDir。setCwd 覆盖后纯 JS。 来源：`packages/node-compat/src/runtime/path.js:45-60,252-304,510-545,737-743` |
| Node API | `path.toNamespacedPath` | JS/浏览器 | — | 是/按分支 | 不做（同步 cwd 查询） | 当前路径：混合(JS+XHR)；Windows 分支调用 resolve 并可能同步取 cwd；POSIX 分支直返输入。_makeLong 在默认对象及 posix/win32 子对象可见，但非 named export。 来源：`packages/node-compat/src/runtime/path.js:547-556,710-727; packages/node-compat/src/path.js:11-23` |
| Node API | `path._makeLong` | JS/浏览器 | — | 是/按分支 | 不做（同步 cwd 查询） | 当前路径：混合(JS+XHR)；Windows 分支调用 resolve 并可能同步取 cwd；POSIX 分支直返输入。_makeLong 在默认对象及 posix/win32 子对象可见，但非 named export。 来源：`packages/node-compat/src/runtime/path.js:547-556,710-727; packages/node-compat/src/path.js:11-23` |
| Node API | `path.setCwd` | JS/浏览器 | — | — | 无需 | 当前路径：JS；NodeCompat 自定义扩展；修改当前 JS path 模块闭包的 cwd override，不发 bridge。 来源：`packages/node-compat/src/runtime/path.js:730-743; packages/node-compat/src/path.js:22-23` |
| Node API | `path.posix` | JS/浏览器 | — | — | 无需 | 当前路径：JS；暴露 POSIX 与 Windows 子模块对象；两者复用同名算法，resolve/relative/toNamespacedPath 仍可能查 cwd。 来源：`packages/node-compat/src/runtime/path.js:730-743; packages/node-compat/src/path.js:18-21` |
| Node API | `path.win32` | JS/浏览器 | — | — | 无需 | 当前路径：JS；暴露 POSIX 与 Windows 子模块对象；两者复用同名算法，resolve/relative/toNamespacedPath 仍可能查 cwd。 来源：`packages/node-compat/src/runtime/path.js:730-743; packages/node-compat/src/path.js:18-21` |
| Node API | `url.URL` | JS/浏览器 | — | — | 无需 | 当前路径：JS；宿主 WHATWG URL 类、legacyUrl 实现及文件 URL 解码均为 JS。parse/format/resolve/resolveObject/Url 在默认 url 与 ESM wrapper 导出，未列入旧4项。 来源：`packages/node-compat/src/runtime/url.js:83-109,140-152; packages/node-compat/src/url.js:6-17` |
| Node API | `url.URLSearchParams` | JS/浏览器 | — | — | 无需 | 当前路径：JS；宿主 WHATWG URL 类、legacyUrl 实现及文件 URL 解码均为 JS。parse/format/resolve/resolveObject/Url 在默认 url 与 ESM wrapper 导出，未列入旧4项。 来源：`packages/node-compat/src/runtime/url.js:83-109,140-152; packages/node-compat/src/url.js:6-17` |
| Node API | `url.parse` | JS/浏览器 | — | — | 无需 | 当前路径：JS；宿主 WHATWG URL 类、legacyUrl 实现及文件 URL 解码均为 JS。parse/format/resolve/resolveObject/Url 在默认 url 与 ESM wrapper 导出，未列入旧4项。 来源：`packages/node-compat/src/runtime/url.js:83-109,140-152; packages/node-compat/src/url.js:6-17` |
| Node API | `url.format` | JS/浏览器 | — | — | 无需 | 当前路径：JS；宿主 WHATWG URL 类、legacyUrl 实现及文件 URL 解码均为 JS。parse/format/resolve/resolveObject/Url 在默认 url 与 ESM wrapper 导出，未列入旧4项。 来源：`packages/node-compat/src/runtime/url.js:83-109,140-152; packages/node-compat/src/url.js:6-17` |
| Node API | `url.resolve` | JS/浏览器 | — | — | 无需 | 当前路径：JS；宿主 WHATWG URL 类、legacyUrl 实现及文件 URL 解码均为 JS。parse/format/resolve/resolveObject/Url 在默认 url 与 ESM wrapper 导出，未列入旧4项。 来源：`packages/node-compat/src/runtime/url.js:83-109,140-152; packages/node-compat/src/url.js:6-17` |
| Node API | `url.resolveObject` | JS/浏览器 | — | — | 无需 | 当前路径：JS；宿主 WHATWG URL 类、legacyUrl 实现及文件 URL 解码均为 JS。parse/format/resolve/resolveObject/Url 在默认 url 与 ESM wrapper 导出，未列入旧4项。 来源：`packages/node-compat/src/runtime/url.js:83-109,140-152; packages/node-compat/src/url.js:6-17` |
| Node API | `url.Url` | JS/浏览器 | — | — | 无需 | 当前路径：JS；宿主 WHATWG URL 类、legacyUrl 实现及文件 URL 解码均为 JS。parse/format/resolve/resolveObject/Url 在默认 url 与 ESM wrapper 导出，未列入旧4项。 来源：`packages/node-compat/src/runtime/url.js:83-109,140-152; packages/node-compat/src/url.js:6-17` |
| Node API | `url.fileURLToPath` | JS/浏览器 | — | — | 无需 | 当前路径：JS；宿主 WHATWG URL 类、legacyUrl 实现及文件 URL 解码均为 JS。parse/format/resolve/resolveObject/Url 在默认 url 与 ESM wrapper 导出，未列入旧4项。 来源：`packages/node-compat/src/runtime/url.js:83-109,140-152; packages/node-compat/src/url.js:6-17` |
| Node API | `url.pathToFileURL` | JS/浏览器 | — | 是/按分支 | 不做（同步 cwd 查询） | 当前路径：混合(JS+XHR)；相对路径转绝对路径时调用 path.resolve，当前可能同步 XHR 获取 cwd；绝对路径仍走 JS。 来源：`packages/node-compat/src/runtime/url.js:111-138; packages/node-compat/src/runtime/path.js:49-60,252-267` |
| Node API | `child_process.spawn` | — | 流式 | — | 建议（仅有界文本 exec/execFile） | 当前路径：WS流；spawn 用 process.execStream，子 stdin 是二进制上行、stdout/stderr 是流；exec/execFile 聚合输出且默认 maxBuffer=1 MiB，但仍返回 ChildProcess 事件/流对象。若做 IPC 应是独立有界高层入口，不能伪装 spawn 流。 来源：`packages/node-compat/src/runtime/child_process.js:31-120,122-199; crates/niva/assets/initialize_script.js:538-568` |
| Node API | `child_process.exec` | — | 流式 | — | 建议（仅有界文本 exec/execFile） | 当前路径：WS流；spawn 用 process.execStream，子 stdin 是二进制上行、stdout/stderr 是流；exec/execFile 聚合输出且默认 maxBuffer=1 MiB，但仍返回 ChildProcess 事件/流对象。若做 IPC 应是独立有界高层入口，不能伪装 spawn 流。 来源：`packages/node-compat/src/runtime/child_process.js:31-120,122-199; crates/niva/assets/initialize_script.js:538-568` |
| Node API | `child_process.execFile` | — | 流式 | — | 建议（仅有界文本 exec/execFile） | 当前路径：WS流；spawn 用 process.execStream，子 stdin 是二进制上行、stdout/stderr 是流；exec/execFile 聚合输出且默认 maxBuffer=1 MiB，但仍返回 ChildProcess 事件/流对象。若做 IPC 应是独立有界高层入口，不能伪装 spawn 流。 来源：`packages/node-compat/src/runtime/child_process.js:31-120,122-199; crates/niva/assets/initialize_script.js:538-568` |
| Node API | `ChildProcess.kill` | — | 流式 | — | 不做（持续子进程） | 当前路径：WS流；实例 kill 通过 process.signal stream；实例另暴露 cancel/ref/unref、pid/killed/connected/exitCode/signalCode/spawnfile/spawnargs、stdin/stdout/stderr/stdio、completion；execFile 另有 result Promise。 来源：`packages/node-compat/src/runtime/child_process.js:46-78,105-120,183-194` |
| Node API | `child_process.spawnSync` | — | — | 是/按分支 | 不做（同步/二进制） | 当前路径：XHR；通过 callSync process.spawnSync；stdout/stderr 以 Buffer 或 encoding 返回，跨 Native 同步 IPC 与二进制均排除。 来源：`packages/node-compat/src/runtime/child_process.js:200-245` |
| Node API | `child_process.execFileSync` | — | — | 是/按分支 | 不做（同步/二进制） | 当前路径：XHR；通过 callSync process.spawnSync；stdout/stderr 以 Buffer 或 encoding 返回，跨 Native 同步 IPC 与二进制均排除。 来源：`packages/node-compat/src/runtime/child_process.js:200-245` |
| Node API | `child_process.execSync` | — | — | 是/按分支 | 不做（同步/二进制） | 当前路径：XHR；通过 callSync process.spawnSync；stdout/stderr 以 Buffer 或 encoding 返回，跨 Native 同步 IPC 与二进制均排除。 来源：`packages/node-compat/src/runtime/child_process.js:200-245` |
| Node API | `child_process.ChildProcess instance surface` | JS/浏览器 | 流式 | — | 建议（只限 exec 最终文本） | 当前路径：混合(JS+WS流)；EventEmitter 子进程对象；spawn 是流式任务，execFile/exec 以 maxBuffer=1 MiB 聚合；实例暴露 kill/cancel/ref/unref、stdio 与完成 Promise。 来源：`packages/node-compat/src/runtime/child_process.js:46-120,138-194` |
| Node API | `net.connect` | — | 流式 | — | 不做（TCP socket） | 当前路径：WS流；同一函数别名；连接后返回 Duplex Socket，字节和控制均走 socket.* WS stream。 来源：`packages/node-compat/src/runtime/net.js:14-27,106-144,426-433,649-660; packages/node-compat/src/net.js:9-17` |
| Node API | `net.createConnection` | — | 流式 | — | 不做（TCP socket） | 当前路径：WS流；同一函数别名；连接后返回 Duplex Socket，字节和控制均走 socket.* WS stream。 来源：`packages/node-compat/src/runtime/net.js:14-27,106-144,426-433,649-660; packages/node-compat/src/net.js:9-17` |
| Node API | `net.createServer` | — | 流式 | — | 不做（TCP listener） | 当前路径：WS流；Server.listen 使用 socket.tcpListen，accept 后 socket.tcpAttach；不能用 IPC unary 替代 listener/socket。 来源：`packages/node-compat/src/runtime/net.js:446-481,484-541,603-606; packages/node-compat/src/net.js:9-17` |
| Node API | `net.Socket` | — | 流式 | — | 不做（原始双向字节流） | 当前路径：WS流；extends Duplex；方法 connect/address/pause/resume/setTimeout/setNoDelay/setKeepAlive/ref/unref/hasRef/destroy/write/end。公开属性 readyState/bufferSize/allowHalfOpen/connecting/pending/bytesRead/bytesWritten、本地/远端 address/port/family、encrypted/authorized/authorizationError；setNoDelay/setKeepAlive 明确抛 ENOTSUP。 来源：`packages/node-compat/src/runtime/net.js:49-110,147-189,288-405; packages/node-compat/src/net.js:9` |
| Node API | `net.Server / Server.listen` | — | 流式 | — | 不做（listener/长会话） | 当前路径：WS流；Server extends EventEmitter；listen/address/ref/unref/hasRef/close/getConnections；公开 listening/maxConnections，connection/clientError/listening/close 事件；被 http/https server 复用。 来源：`packages/node-compat/src/runtime/net.js:446-461,463-580; packages/node-compat/src/net.js:10,13` |
| Node API | `net.isIP` | JS/浏览器 | — | — | 无需 | 当前路径：JS；本地字符串解析函数；存在于 default net 对象，但 ESM wrapper 未单独 named-export。 来源：`packages/node-compat/src/runtime/net.js:613-660; packages/node-compat/src/net.js:8-17` |
| Node API | `net.isIPv4` | JS/浏览器 | — | — | 无需 | 当前路径：JS；本地字符串解析函数；存在于 default net 对象，但 ESM wrapper 未单独 named-export。 来源：`packages/node-compat/src/runtime/net.js:613-660; packages/node-compat/src/net.js:8-17` |
| Node API | `net.isIPv6` | JS/浏览器 | — | — | 无需 | 当前路径：JS；本地字符串解析函数；存在于 default net 对象，但 ESM wrapper 未单独 named-export。 来源：`packages/node-compat/src/runtime/net.js:613-660; packages/node-compat/src/net.js:8-17` |
| Node API | `net.connectGuarded` | — | 流式 | — | 不做（socket/listener） | 当前路径：WS流；runtime default net 对象额外暴露的实现入口，ESM wrapper 未 named-export；分别建立受限 TCP client 与 TLS Server。 来源：`packages/node-compat/src/runtime/net.js:435-444,608-611,649-660` |
| Node API | `net.createTlsServer` | — | 流式 | — | 不做（socket/listener） | 当前路径：WS流；runtime default net 对象额外暴露的实现入口，ESM wrapper 未 named-export；分别建立受限 TCP client 与 TLS Server。 来源：`packages/node-compat/src/runtime/net.js:435-444,608-611,649-660` |
| Node API | `tls.connect` | — | 流式 | — | 不做（TLS socket） | 当前路径：WS流；返回 net.Socket 派生 TLS socket；原始 TLS 会话走 socket.tlsConnect。 来源：`packages/node-compat/src/runtime/tls.js:80-113,210-216; packages/node-compat/src/tls.js:9-14` |
| Node API | `tls.createServer` | — | 流式 | — | 不做（TLS listener/socket） | 当前路径：WS流；TLS Server 复用 net.Server.listen；identity/cert/key 转 Native 后仍是持续 socket 流。 来源：`packages/node-compat/src/runtime/tls.js:147-207; packages/node-compat/src/runtime/net.js:446-481` |
| Node API | `tls.TLSSocket` | — | 流式 | — | 不做（TLS socket/listener） | 当前路径：WS流；TLSSocket 是 net.Socket 别名，Server 是 net.Server 别名；connectGuarded 仅在 default module 对象可见，wrapper 未 named-export。 来源：`packages/node-compat/src/runtime/tls.js:210-216; packages/node-compat/src/tls.js:9-14` |
| Node API | `tls.Server` | — | 流式 | — | 不做（TLS socket/listener） | 当前路径：WS流；TLSSocket 是 net.Socket 别名，Server 是 net.Server 别名；connectGuarded 仅在 default module 对象可见，wrapper 未 named-export。 来源：`packages/node-compat/src/runtime/tls.js:210-216; packages/node-compat/src/tls.js:9-14` |
| Node API | `tls.connectGuarded` | — | 流式 | — | 不做（TLS socket/listener） | 当前路径：WS流；TLSSocket 是 net.Socket 别名，Server 是 net.Server 别名；connectGuarded 仅在 default module 对象可见，wrapper 未 named-export。 来源：`packages/node-compat/src/runtime/tls.js:210-216; packages/node-compat/src/tls.js:9-14` |
| Node API | `http.request` | — | 流式 | — | 不支持当前流式契约；D08 另需一次性 HTTP/HTTPS 高层入口 | 当前路径：WS流；当前返回 ClientRequest stream object，真实 TCP/TLS socket 上写请求并流式解析 IncomingMessage；D08 要求 HTTP/HTTPS 文本/JSON unary fallback；Node request/get 的流式对象契约本身不支持 IPC，另需一次性高层入口承接。 来源：`packages/node-compat/src/runtime/http.js:207-269; packages/node-compat/src/http.js:8-17; packages/node-compat/src/https.js:8-17; docs/architecture-review.md:540-545,560-567` |
| Node API | `http.get` | — | 流式 | — | 不支持当前流式契约；D08 另需一次性 HTTP/HTTPS 高层入口 | 当前路径：WS流；当前返回 ClientRequest stream object，真实 TCP/TLS socket 上写请求并流式解析 IncomingMessage；D08 要求 HTTP/HTTPS 文本/JSON unary fallback；Node request/get 的流式对象契约本身不支持 IPC，另需一次性高层入口承接。 来源：`packages/node-compat/src/runtime/http.js:207-269; packages/node-compat/src/http.js:8-17; packages/node-compat/src/https.js:8-17; docs/architecture-review.md:540-545,560-567` |
| Node API | `https.request` | — | 流式 | — | 不支持当前流式契约；D08 另需一次性 HTTP/HTTPS 高层入口 | 当前路径：WS流；当前返回 ClientRequest stream object，真实 TCP/TLS socket 上写请求并流式解析 IncomingMessage；D08 要求 HTTP/HTTPS 文本/JSON unary fallback；Node request/get 的流式对象契约本身不支持 IPC，另需一次性高层入口承接。 来源：`packages/node-compat/src/runtime/http.js:207-269; packages/node-compat/src/http.js:8-17; packages/node-compat/src/https.js:8-17; docs/architecture-review.md:540-545,560-567` |
| Node API | `https.get` | — | 流式 | — | 不支持当前流式契约；D08 另需一次性 HTTP/HTTPS 高层入口 | 当前路径：WS流；当前返回 ClientRequest stream object，真实 TCP/TLS socket 上写请求并流式解析 IncomingMessage；D08 要求 HTTP/HTTPS 文本/JSON unary fallback；Node request/get 的流式对象契约本身不支持 IPC，另需一次性高层入口承接。 来源：`packages/node-compat/src/runtime/http.js:207-269; packages/node-compat/src/http.js:8-17; packages/node-compat/src/https.js:8-17; docs/architecture-review.md:540-545,560-567` |
| Node API | `http.createServer` | — | 流式 | — | 不做（server/socket持续会话） | 当前路径：WS流；createServer 返回 net.Server；listen 和请求/响应传输均为 WS stream。Node HTTP server 不属于一次性客户端 HTTP fallback。 来源：`packages/node-compat/src/runtime/http.js:270-289; packages/node-compat/src/runtime/net.js:446-580` |
| Node API | `https.createServer` | — | 流式 | — | 不做（server/socket持续会话） | 当前路径：WS流；createServer 返回 net.Server；listen 和请求/响应传输均为 WS stream。Node HTTP server 不属于一次性客户端 HTTP fallback。 来源：`packages/node-compat/src/runtime/http.js:270-289; packages/node-compat/src/runtime/net.js:446-580` |
| Node API | `http.Server.listen` | — | 流式 | — | 不做（server/socket持续会话） | 当前路径：WS流；createServer 返回 net.Server；listen 和请求/响应传输均为 WS stream。Node HTTP server 不属于一次性客户端 HTTP fallback。 来源：`packages/node-compat/src/runtime/http.js:270-289; packages/node-compat/src/runtime/net.js:446-580` |
| Node API | `http.IncomingMessage` | — | 流式 | — | 不做（消息体流） | 当前路径：WS流；Readable；公开 socket/connection、headers/rawHeaders/trailers/rawTrailers、complete/aborted、HTTP 版本字段及 request method/url 或 response statusCode/statusMessage；setTimeout。 来源：`packages/node-compat/src/runtime/http.js:29-35,122-162; packages/node-compat/src/http.js:12; packages/node-compat/src/https.js:12` |
| Node API | `https.IncomingMessage` | — | 流式 | — | 不做（消息体流） | 当前路径：WS流；Readable；公开 socket/connection、headers/rawHeaders/trailers/rawTrailers、complete/aborted、HTTP 版本字段及 request method/url 或 response statusCode/statusMessage；setTimeout。 来源：`packages/node-compat/src/runtime/http.js:29-35,122-162; packages/node-compat/src/http.js:12; packages/node-compat/src/https.js:12` |
| Node API | `http.ClientRequest` | — | 流式 | — | 不支持当前流式契约；D08 另需一次性 HTTP/HTTPS 高层入口 | 当前路径：WS流；OutgoingMessage 派生；公开 method/path/host/protocol/aborted/res，abort；继承 write/end 和 header/trailer 方法，绑定 socket/response/close/timeout 事件。现存流对象本身不切 IPC。 来源：`packages/node-compat/src/runtime/http.js:36-106,209-267; packages/node-compat/src/http.js:14; packages/node-compat/src/https.js:14` |
| Node API | `https.ClientRequest` | — | 流式 | — | 不支持当前流式契约；D08 另需一次性 HTTP/HTTPS 高层入口 | 当前路径：WS流；OutgoingMessage 派生；公开 method/path/host/protocol/aborted/res，abort；继承 write/end 和 header/trailer 方法，绑定 socket/response/close/timeout 事件。现存流对象本身不切 IPC。 来源：`packages/node-compat/src/runtime/http.js:36-106,209-267; packages/node-compat/src/http.js:14; packages/node-compat/src/https.js:14` |
| Node API | `http.ServerResponse` | — | 流式 | — | 不做（服务端响应流） | 当前路径：WS流；OutgoingMessage 派生；公开 req/statusCode/statusMessage/sendDate/writeHead/writeContinue；继承 write/end/header/trailer 方法。 来源：`packages/node-compat/src/runtime/http.js:36-120; packages/node-compat/src/http.js:13; packages/node-compat/src/https.js:13` |
| Node API | `https.ServerResponse` | — | 流式 | — | 不做（服务端响应流） | 当前路径：WS流；OutgoingMessage 派生；公开 req/statusCode/statusMessage/sendDate/writeHead/writeContinue；继承 write/end/header/trailer 方法。 来源：`packages/node-compat/src/runtime/http.js:36-120; packages/node-compat/src/http.js:13; packages/node-compat/src/https.js:13` |
| Node API | `http.METHODS` | 静态快照/常量 | — | — | 无需 | 当前路径：静态；JS 常量表/数组；STATUS_CODES 是实现内的选定子集。 来源：`packages/node-compat/src/runtime/http.js:5-6,289; packages/node-compat/src/http.js:15-16; packages/node-compat/src/https.js:15-16` |
| Node API | `https.METHODS` | 静态快照/常量 | — | — | 无需 | 当前路径：静态；JS 常量表/数组；STATUS_CODES 是实现内的选定子集。 来源：`packages/node-compat/src/runtime/http.js:5-6,289; packages/node-compat/src/http.js:15-16; packages/node-compat/src/https.js:15-16` |
| Node API | `http.STATUS_CODES` | 静态快照/常量 | — | — | 无需 | 当前路径：静态；JS 常量表/数组；STATUS_CODES 是实现内的选定子集。 来源：`packages/node-compat/src/runtime/http.js:5-6,289; packages/node-compat/src/http.js:15-16; packages/node-compat/src/https.js:15-16` |
| Node API | `https.STATUS_CODES` | 静态快照/常量 | — | — | 无需 | 当前路径：静态；JS 常量表/数组；STATUS_CODES 是实现内的选定子集。 来源：`packages/node-compat/src/runtime/http.js:5-6,289; packages/node-compat/src/http.js:15-16; packages/node-compat/src/https.js:15-16` |
| Node API | `http.Server instance surface` | — | 流式 | — | 不做（服务端长会话） | 当前路径：WS流；返回的 net.Server 提供 listen/address/ref/unref/hasRef/close/getConnections、listening/maxConnections 和事件接口。 来源：`packages/node-compat/src/runtime/http.js:270-287; packages/node-compat/src/runtime/net.js:446-580` |
| Node API | `https.Server instance surface` | — | 流式 | — | 不做（服务端长会话） | 当前路径：WS流；返回的 net.Server 提供 listen/address/ref/unref/hasRef/close/getConnections、listening/maxConnections 和事件接口。 来源：`packages/node-compat/src/runtime/http.js:270-287; packages/node-compat/src/runtime/net.js:446-580` |
| Node API | `dns.lookup` | — | 异步 | — | 待核（一次性 DNS JSON 查询是否纳入网络 fallback） | 当前路径：WS；callback 与 promises.lookup 共用 os.dnsLookup unary；默认返回 address/family，对 all:true 返回列表。 来源：`packages/node-compat/src/runtime/dns.js:355-384,389-405; packages/node-compat/src/dns.js:5-19; packages/node-compat/src/dns-promises.js:5-16` |
| Node API | `dns.resolve` | JS/浏览器 | 流式 | 是/按分支 | 待核（需单次 Native DNS unary；不得复用 socket 流作 IPC） | 当前路径：混合(JS+XHR+WS流)；JS 编解码 DNS；默认 getServers 通过 os.dnsServers callSync，查询走 UDP datagram，截断时走 TCP socket；promises 包装相同查询。D08 明确一次性 HTTP，DNS fallback 清单待定。 来源：`packages/node-compat/src/runtime/dns.js:94-102,114-191,220-259,262-278,297-405; packages/node-compat/src/dns.js:5-19; packages/node-compat/src/dns-promises.js:5-16` |
| Node API | `dns.resolve4` | JS/浏览器 | 流式 | 是/按分支 | 待核（需单次 Native DNS unary；不得复用 socket 流作 IPC） | 当前路径：混合(JS+XHR+WS流)；JS 编解码 DNS；默认 getServers 通过 os.dnsServers callSync，查询走 UDP datagram，截断时走 TCP socket；promises 包装相同查询。D08 明确一次性 HTTP，DNS fallback 清单待定。 来源：`packages/node-compat/src/runtime/dns.js:94-102,114-191,220-259,262-278,297-405; packages/node-compat/src/dns.js:5-19; packages/node-compat/src/dns-promises.js:5-16` |
| Node API | `dns.resolve6` | JS/浏览器 | 流式 | 是/按分支 | 待核（需单次 Native DNS unary；不得复用 socket 流作 IPC） | 当前路径：混合(JS+XHR+WS流)；JS 编解码 DNS；默认 getServers 通过 os.dnsServers callSync，查询走 UDP datagram，截断时走 TCP socket；promises 包装相同查询。D08 明确一次性 HTTP，DNS fallback 清单待定。 来源：`packages/node-compat/src/runtime/dns.js:94-102,114-191,220-259,262-278,297-405; packages/node-compat/src/dns.js:5-19; packages/node-compat/src/dns-promises.js:5-16` |
| Node API | `dns.resolveCname` | JS/浏览器 | 流式 | 是/按分支 | 待核（需单次 Native DNS unary；不得复用 socket 流作 IPC） | 当前路径：混合(JS+XHR+WS流)；JS 编解码 DNS；默认 getServers 通过 os.dnsServers callSync，查询走 UDP datagram，截断时走 TCP socket；promises 包装相同查询。D08 明确一次性 HTTP，DNS fallback 清单待定。 来源：`packages/node-compat/src/runtime/dns.js:94-102,114-191,220-259,262-278,297-405; packages/node-compat/src/dns.js:5-19; packages/node-compat/src/dns-promises.js:5-16` |
| Node API | `dns.resolveMx` | JS/浏览器 | 流式 | 是/按分支 | 待核（需单次 Native DNS unary；不得复用 socket 流作 IPC） | 当前路径：混合(JS+XHR+WS流)；JS 编解码 DNS；默认 getServers 通过 os.dnsServers callSync，查询走 UDP datagram，截断时走 TCP socket；promises 包装相同查询。D08 明确一次性 HTTP，DNS fallback 清单待定。 来源：`packages/node-compat/src/runtime/dns.js:94-102,114-191,220-259,262-278,297-405; packages/node-compat/src/dns.js:5-19; packages/node-compat/src/dns-promises.js:5-16` |
| Node API | `dns.resolveTxt` | JS/浏览器 | 流式 | 是/按分支 | 待核（需单次 Native DNS unary；不得复用 socket 流作 IPC） | 当前路径：混合(JS+XHR+WS流)；JS 编解码 DNS；默认 getServers 通过 os.dnsServers callSync，查询走 UDP datagram，截断时走 TCP socket；promises 包装相同查询。D08 明确一次性 HTTP，DNS fallback 清单待定。 来源：`packages/node-compat/src/runtime/dns.js:94-102,114-191,220-259,262-278,297-405; packages/node-compat/src/dns.js:5-19; packages/node-compat/src/dns-promises.js:5-16` |
| Node API | `dns.resolveNs` | JS/浏览器 | 流式 | 是/按分支 | 待核（需单次 Native DNS unary；不得复用 socket 流作 IPC） | 当前路径：混合(JS+XHR+WS流)；JS 编解码 DNS；默认 getServers 通过 os.dnsServers callSync，查询走 UDP datagram，截断时走 TCP socket；promises 包装相同查询。D08 明确一次性 HTTP，DNS fallback 清单待定。 来源：`packages/node-compat/src/runtime/dns.js:94-102,114-191,220-259,262-278,297-405; packages/node-compat/src/dns.js:5-19; packages/node-compat/src/dns-promises.js:5-16` |
| Node API | `dns.resolveSrv` | JS/浏览器 | 流式 | 是/按分支 | 待核（需单次 Native DNS unary；不得复用 socket 流作 IPC） | 当前路径：混合(JS+XHR+WS流)；JS 编解码 DNS；默认 getServers 通过 os.dnsServers callSync，查询走 UDP datagram，截断时走 TCP socket；promises 包装相同查询。D08 明确一次性 HTTP，DNS fallback 清单待定。 来源：`packages/node-compat/src/runtime/dns.js:94-102,114-191,220-259,262-278,297-405; packages/node-compat/src/dns.js:5-19; packages/node-compat/src/dns-promises.js:5-16` |
| Node API | `dns.resolveSoa` | JS/浏览器 | 流式 | 是/按分支 | 待核（需单次 Native DNS unary；不得复用 socket 流作 IPC） | 当前路径：混合(JS+XHR+WS流)；JS 编解码 DNS；默认 getServers 通过 os.dnsServers callSync，查询走 UDP datagram，截断时走 TCP socket；promises 包装相同查询。D08 明确一次性 HTTP，DNS fallback 清单待定。 来源：`packages/node-compat/src/runtime/dns.js:94-102,114-191,220-259,262-278,297-405; packages/node-compat/src/dns.js:5-19; packages/node-compat/src/dns-promises.js:5-16` |
| Node API | `dns.resolvePtr` | JS/浏览器 | 流式 | 是/按分支 | 待核（需单次 Native DNS unary；不得复用 socket 流作 IPC） | 当前路径：混合(JS+XHR+WS流)；JS 编解码 DNS；默认 getServers 通过 os.dnsServers callSync，查询走 UDP datagram，截断时走 TCP socket；promises 包装相同查询。D08 明确一次性 HTTP，DNS fallback 清单待定。 来源：`packages/node-compat/src/runtime/dns.js:94-102,114-191,220-259,262-278,297-405; packages/node-compat/src/dns.js:5-19; packages/node-compat/src/dns-promises.js:5-16` |
| Node API | `dns/promises.resolve` | JS/浏览器 | 流式 | 是/按分支 | 待核（需单次 Native DNS unary；不得复用 socket 流作 IPC） | 当前路径：混合(JS+XHR+WS流)；JS 编解码 DNS；默认 getServers 通过 os.dnsServers callSync，查询走 UDP datagram，截断时走 TCP socket；promises 包装相同查询。D08 明确一次性 HTTP，DNS fallback 清单待定。 来源：`packages/node-compat/src/runtime/dns.js:94-102,114-191,220-259,262-278,297-405; packages/node-compat/src/dns.js:5-19; packages/node-compat/src/dns-promises.js:5-16` |
| Node API | `dns/promises.resolve4` | JS/浏览器 | 流式 | 是/按分支 | 待核（需单次 Native DNS unary；不得复用 socket 流作 IPC） | 当前路径：混合(JS+XHR+WS流)；JS 编解码 DNS；默认 getServers 通过 os.dnsServers callSync，查询走 UDP datagram，截断时走 TCP socket；promises 包装相同查询。D08 明确一次性 HTTP，DNS fallback 清单待定。 来源：`packages/node-compat/src/runtime/dns.js:94-102,114-191,220-259,262-278,297-405; packages/node-compat/src/dns.js:5-19; packages/node-compat/src/dns-promises.js:5-16` |
| Node API | `dns/promises.resolve6` | JS/浏览器 | 流式 | 是/按分支 | 待核（需单次 Native DNS unary；不得复用 socket 流作 IPC） | 当前路径：混合(JS+XHR+WS流)；JS 编解码 DNS；默认 getServers 通过 os.dnsServers callSync，查询走 UDP datagram，截断时走 TCP socket；promises 包装相同查询。D08 明确一次性 HTTP，DNS fallback 清单待定。 来源：`packages/node-compat/src/runtime/dns.js:94-102,114-191,220-259,262-278,297-405; packages/node-compat/src/dns.js:5-19; packages/node-compat/src/dns-promises.js:5-16` |
| Node API | `dns/promises.resolveCname` | JS/浏览器 | 流式 | 是/按分支 | 待核（需单次 Native DNS unary；不得复用 socket 流作 IPC） | 当前路径：混合(JS+XHR+WS流)；JS 编解码 DNS；默认 getServers 通过 os.dnsServers callSync，查询走 UDP datagram，截断时走 TCP socket；promises 包装相同查询。D08 明确一次性 HTTP，DNS fallback 清单待定。 来源：`packages/node-compat/src/runtime/dns.js:94-102,114-191,220-259,262-278,297-405; packages/node-compat/src/dns.js:5-19; packages/node-compat/src/dns-promises.js:5-16` |
| Node API | `dns/promises.resolveMx` | JS/浏览器 | 流式 | 是/按分支 | 待核（需单次 Native DNS unary；不得复用 socket 流作 IPC） | 当前路径：混合(JS+XHR+WS流)；JS 编解码 DNS；默认 getServers 通过 os.dnsServers callSync，查询走 UDP datagram，截断时走 TCP socket；promises 包装相同查询。D08 明确一次性 HTTP，DNS fallback 清单待定。 来源：`packages/node-compat/src/runtime/dns.js:94-102,114-191,220-259,262-278,297-405; packages/node-compat/src/dns.js:5-19; packages/node-compat/src/dns-promises.js:5-16` |
| Node API | `dns/promises.resolveTxt` | JS/浏览器 | 流式 | 是/按分支 | 待核（需单次 Native DNS unary；不得复用 socket 流作 IPC） | 当前路径：混合(JS+XHR+WS流)；JS 编解码 DNS；默认 getServers 通过 os.dnsServers callSync，查询走 UDP datagram，截断时走 TCP socket；promises 包装相同查询。D08 明确一次性 HTTP，DNS fallback 清单待定。 来源：`packages/node-compat/src/runtime/dns.js:94-102,114-191,220-259,262-278,297-405; packages/node-compat/src/dns.js:5-19; packages/node-compat/src/dns-promises.js:5-16` |
| Node API | `dns/promises.resolveNs` | JS/浏览器 | 流式 | 是/按分支 | 待核（需单次 Native DNS unary；不得复用 socket 流作 IPC） | 当前路径：混合(JS+XHR+WS流)；JS 编解码 DNS；默认 getServers 通过 os.dnsServers callSync，查询走 UDP datagram，截断时走 TCP socket；promises 包装相同查询。D08 明确一次性 HTTP，DNS fallback 清单待定。 来源：`packages/node-compat/src/runtime/dns.js:94-102,114-191,220-259,262-278,297-405; packages/node-compat/src/dns.js:5-19; packages/node-compat/src/dns-promises.js:5-16` |
| Node API | `dns/promises.resolveSrv` | JS/浏览器 | 流式 | 是/按分支 | 待核（需单次 Native DNS unary；不得复用 socket 流作 IPC） | 当前路径：混合(JS+XHR+WS流)；JS 编解码 DNS；默认 getServers 通过 os.dnsServers callSync，查询走 UDP datagram，截断时走 TCP socket；promises 包装相同查询。D08 明确一次性 HTTP，DNS fallback 清单待定。 来源：`packages/node-compat/src/runtime/dns.js:94-102,114-191,220-259,262-278,297-405; packages/node-compat/src/dns.js:5-19; packages/node-compat/src/dns-promises.js:5-16` |
| Node API | `dns/promises.resolveSoa` | JS/浏览器 | 流式 | 是/按分支 | 待核（需单次 Native DNS unary；不得复用 socket 流作 IPC） | 当前路径：混合(JS+XHR+WS流)；JS 编解码 DNS；默认 getServers 通过 os.dnsServers callSync，查询走 UDP datagram，截断时走 TCP socket；promises 包装相同查询。D08 明确一次性 HTTP，DNS fallback 清单待定。 来源：`packages/node-compat/src/runtime/dns.js:94-102,114-191,220-259,262-278,297-405; packages/node-compat/src/dns.js:5-19; packages/node-compat/src/dns-promises.js:5-16` |
| Node API | `dns/promises.resolvePtr` | JS/浏览器 | 流式 | 是/按分支 | 待核（需单次 Native DNS unary；不得复用 socket 流作 IPC） | 当前路径：混合(JS+XHR+WS流)；JS 编解码 DNS；默认 getServers 通过 os.dnsServers callSync，查询走 UDP datagram，截断时走 TCP socket；promises 包装相同查询。D08 明确一次性 HTTP，DNS fallback 清单待定。 来源：`packages/node-compat/src/runtime/dns.js:94-102,114-191,220-259,262-278,297-405; packages/node-compat/src/dns.js:5-19; packages/node-compat/src/dns-promises.js:5-16` |
| Node API | `dns.Resolver` | JS/浏览器 | 流式 | 是/按分支 | 待核 | 当前路径：混合(JS+XHR+WS流)；实例 timeout/tries 为 JS 属性；setServers 是 JS override；getServers 默认同步 XHR 取 OS resolver；resolve* 走 UDP/TCP WS stream。 来源：`packages/node-compat/src/runtime/dns.js:280-338,385-405; packages/node-compat/src/dns.js:16-19; packages/node-compat/src/dns-promises.js:16` |
| Node API | `dns/promises.Resolver` | JS/浏览器 | 流式 | 是/按分支 | 待核 | 当前路径：混合(JS+XHR+WS流)；实例 timeout/tries 为 JS 属性；setServers 是 JS override；getServers 默认同步 XHR 取 OS resolver；resolve* 走 UDP/TCP WS stream。 来源：`packages/node-compat/src/runtime/dns.js:280-338,385-405; packages/node-compat/src/dns.js:16-19; packages/node-compat/src/dns-promises.js:16` |
| Node API | `dns.setServers` | JS/浏览器 | — | — | 无需 | 当前路径：JS；callback 模块 default Resolver 的本地服务器覆盖配置；promises 子模块不导出 setServers。dns/promises.setServers 未实际公开，旧清单无此项；标记需核。 来源：`packages/node-compat/src/runtime/dns.js:290-295,389-405; packages/node-compat/src/dns.js:16-19; packages/node-compat/src/dns-promises.js:4-17` |
| Node API | `dns.getServers` | JS/浏览器 | — | 是/按分支 | 不做（当前同步 getter） | 当前路径：混合(JS+XHR)；default Resolver 有 override 时只读本地 JS；否则 callSync os.dnsServers。dns-promises wrapper 不导出 getServers。 来源：`packages/node-compat/src/runtime/dns.js:94-102,290-295,389-405; packages/node-compat/src/dns.js:16-19` |
| Node API | `dns.promises` | JS/浏览器 | 流式 | 是/按分支 | 待核 | 当前路径：混合(JS+XHR+WS流)；子对象含 lookup/resolve/resolve4/resolve6/resolveCname/resolveMx/resolveTxt/resolveNs/resolveSrv/resolveSoa/resolvePtr 与 Resolver；各自沿用上述 callback 底层路线。 来源：`packages/node-compat/src/runtime/dns.js:338-405; packages/node-compat/src/runtime/registration.js:96-100` |
| Node API | `dgram.createSocket` | — | 流式 | — | 不做（二进制 datagram） | 当前路径：WS流；返回 UDP Socket，bind/send/close 控制走 socket.* stream，message 收原始 Buffer/blob。 来源：`packages/node-compat/src/runtime/dgram.js:12-19,26-49,51-112,119-219; packages/node-compat/src/dgram.js:7-10` |
| Node API | `dgram.Socket instance surface` | — | 流式 | — | 不做（二进制 datagram） | 当前路径：WS流；extends EventEmitter；type；bind/address/send/close/ref/unref/hasRef/setBroadcast/addMembership/dropMembership/setTTL/setMulticastTTL/setMulticastLoopback。broadcast/multicast/TTL 相关方法明确部分或全部 ENOTSUP。 来源：`packages/node-compat/src/runtime/dgram.js:26-49,51-116,119-219` |
| Node API | `fs.cp` | JS/浏览器 | 异步 | — | 必做（由 Native 一次性复制承接） | 当前路径：混合(JS+WS)；callback 形式包装 promises.cp；当前在 JS 递归遍历目录并串行调用 lstat/mkdir/readdir/copyFile。D08 目标为内容不经 IPC 的 Native 端一次性复制。 来源：`packages/node-compat/src/runtime/fs.js:121-142` |
| Node API | `fs/promises.cp` | JS/浏览器 | 异步 | — | 必做（由 Native 一次性复制承接） | 当前路径：混合(JS+WS)；Promise 形式；当前 JS 递归遍历目录并串行调用其他 fs promises 方法。D08 要求文件复制 Native 内部完成，文件内容不经过 IPC。 来源：`packages/node-compat/src/runtime/fs.js:121-137; docs/architecture-review.md:550-569` |
| Node API | `fs.promises / fs/promises.constants` | 静态快照/常量 | — | — | 无需 | 当前路径：静态；fs 默认对象的 promises 子对象及其 constants 字段；fs/promises 子路径也公开 constants。promises.open 另列为 WS流。 来源：`packages/node-compat/src/runtime/fs.js:96,151-152,285-286; packages/node-compat/src/fs.js:39-43; packages/node-compat/src/fs-promises.js:9-26` |
| Node API | `dns/promises.lookup` | — | 异步 | — | 待核（一次性 DNS JSON 查询是否纳入网络 fallback） | 当前路径：WS；当前 Promise API 包装 callback lookup，最终走 os.dnsLookup unary WS；all 选项返回 JSON 地址列表。dns-promises.js 明确导出 lookup。 来源：`packages/node-compat/src/runtime/dns.js:340-353,355-384; packages/node-compat/src/dns-promises.js:4-16` |
| Node API | `fs.promises` | JS/浏览器 | — | — | 无需 | 当前路径：JS；fs 默认对象公开 promises 子对象；fs/promises 注册子路径另行列出。其 open 为 WS流，其他 promise methods 按各自记录。 来源：`packages/node-compat/src/runtime/fs.js:96,285-286; packages/node-compat/src/fs.js:39-43` |
| Node API | `fs/promises.constants` | 静态快照/常量 | — | — | 无需 | 当前路径：静态；fs/promises 子路径导出的常量对象，与 fs.constants 共用。 来源：`packages/node-compat/src/runtime/fs.js:7,96; packages/node-compat/src/fs-promises.js:9-10` |
| Node API | `events.EventEmitter`<br>`events.default (EventEmitter)` | JS/浏览器 | — | — | 不需要fallback（纯JS事件对象） | 当前路径：JS；ESM 默认出口与 named EventEmitter 指向同一构造器。 来源：`packages/node-compat/src/events.js`；`packages/node-compat/src/runtime/events.js` |
| Node API | `events.EventEmitter.EventEmitter`<br>`events.EventEmitter.defaultMaxListeners`<br>`events.EventEmitter.captureRejections`<br>`events.EventEmitter.errorMonitor`<br>`events.EventEmitter.listenerCount(emitter,event)`<br>`events.defaultMaxListeners`<br>`events.errorMonitor` | JS/浏览器 | — | — | 不需要fallback（纯JS静态属性/事件辅助函数） | 当前路径：JS；包含 EventEmitter 构造器静态属性及 named 导出别名；defaultMaxListeners 是本地可变默认值，不是 Native 静态注入。 来源：`packages/node-compat/src/runtime/events.js`；`packages/node-compat/src/events.js` |
| Node API | `events.EventEmitter.emit`<br>`events.EventEmitter.addListener`<br>`events.EventEmitter.on`<br>`events.EventEmitter.prependListener`<br>`events.EventEmitter.once`<br>`events.EventEmitter.prependOnceListener`<br>`events.EventEmitter.removeListener`<br>`events.EventEmitter.off`<br>`events.EventEmitter.removeAllListeners`<br>`events.EventEmitter.listeners`<br>`events.EventEmitter.rawListeners`<br>`events.EventEmitter.listenerCount`<br>`events.EventEmitter.eventNames`<br>`events.EventEmitter.setMaxListeners`<br>`events.EventEmitter.getMaxListeners` | JS/浏览器 | — | — | 不需要fallback（纯JS事件对象） | 当前路径：JS；EventEmitter 实例方法，名称逐项列出；监听器集合在页面内存中管理，不等于 Bridge 事件订阅。 来源：`packages/node-compat/src/runtime/events.js` |
| Node API | `events.once`<br>`events.on` | JS/浏览器 | — | — | 不需要fallback（纯JS事件工具） | 当前路径：JS；once 返回 Promise；on 返回本地异步迭代器，可观察 EventEmitter/EventTarget，不执行 IPC 或 WS 流传输。 来源：`packages/node-compat/src/events.js`；`packages/node-compat/src/runtime/events.js` |
| Node API | `util.format`<br>`util.formatWithOptions`<br>`util.inspect`<br>`util.promisify`<br>`util.callbackify`<br>`util.isDeepStrictEqual`<br>`util.deprecate`<br>`util.default (module object)` | JS/浏览器 | — | — | 不需要fallback（纯JS工具） | 当前路径：JS；对应 ESM named exports；inspect 使用本地格式化实现，promisify/callbackify 只包装本地函数。 来源：`packages/node-compat/src/util.js`；`packages/node-compat/src/runtime/util.js` |
| Node API | `util.isDeepEqual`<br>`util.isPartialDeepStrictEqual`<br>`util.customPromisifyArgs`<br>`util.promisify.custom`<br>`util.promisify.customArgs`<br>`util.inspect.defaultOptions` | JS/浏览器 | — | — | 不需要fallback（纯JS工具） | 当前路径：JS；默认导出对象还含这些属性；其中 inspect.defaultOptions/promisify 符号由页面本地函数对象提供。它们不是 util.js 的 named exports。 来源：`packages/node-compat/src/runtime/util.js`；`packages/node-compat/src/util.js` |
| Node API | `buffer.Buffer`<br>`buffer.SlowBuffer`<br>`buffer.INSPECT_MAX_BYTES`<br>`buffer.kMaxLength`<br>`buffer.default.constants.MAX_LENGTH`<br>`buffer.default.constants.MAX_STRING_LENGTH`<br>`buffer.default (module object)` | JS/浏览器 | — | — | 不需要fallback（纯JS Buffer/TypedArray） | 当前路径：JS；ESM named exports 加默认模块对象的 constants；Buffer 类来自 browser buffer vendor，运行时适配器设置上限和常量。 来源：`packages/node-compat/src/buffer.js`；`packages/node-compat/src/runtime/buffer.js` |
| Node API | `Buffer.from`<br>`Buffer.alloc`<br>`Buffer.allocUnsafe`<br>`Buffer.allocUnsafeSlow`<br>`Buffer.isBuffer`<br>`Buffer.isEncoding`<br>`Buffer.byteLength`<br>`Buffer.compare`<br>`Buffer.concat`<br>`Buffer.copyBytesFrom` | JS/浏览器 | — | — | 不需要fallback（纯JS二进制处理；字节不会跨Native传输） | 当前路径：JS；Buffer 静态工厂、检查、比较、拼接方法；copyBytesFrom 由 runtime/buffer.js 实现。Binary 值留在 JS 内，不表示支持 IPC 二进制 fallback。 来源：`packages/node-compat/src/runtime/buffer.js`；`packages/node-compat/src/vendor/index.js` |
| Node API | `Buffer.prototype.toString`<br>`Buffer.prototype.toLocaleString`<br>`Buffer.prototype.write`<br>`Buffer.prototype.fill`<br>`Buffer.prototype.compare`<br>`Buffer.prototype.copy`<br>`Buffer.prototype.equals`<br>`Buffer.prototype.indexOf`<br>`Buffer.prototype.lastIndexOf`<br>`Buffer.prototype.includes`<br>`Buffer.prototype.swap16`<br>`Buffer.prototype.swap32`<br>`Buffer.prototype.swap64`<br>`Buffer.prototype.toJSON`<br>`Buffer.prototype.inspect` | JS/浏览器 | — | — | 不需要fallback（纯JS Buffer/TypedArray） | 当前路径：JS；Buffer 实例的文本、查找、比较、复制、填充与字节序转换方法；部分入口由 runtime/buffer.js 包装 vendor 实现。 来源：`packages/node-compat/src/runtime/buffer.js`；`packages/node-compat/src/vendor/index.js` |
| Node API | `Buffer.prototype.readInt8`<br>`Buffer.prototype.readUInt8`<br>`Buffer.prototype.readInt16LE`<br>`Buffer.prototype.readInt16BE`<br>`Buffer.prototype.readUInt16LE`<br>`Buffer.prototype.readUInt16BE`<br>`Buffer.prototype.readInt32LE`<br>`Buffer.prototype.readInt32BE`<br>`Buffer.prototype.readUInt32LE`<br>`Buffer.prototype.readUInt32BE`<br>`Buffer.prototype.readIntLE`<br>`Buffer.prototype.readIntBE`<br>`Buffer.prototype.readUIntLE`<br>`Buffer.prototype.readUIntBE`<br>`Buffer.prototype.readFloatLE`<br>`Buffer.prototype.readFloatBE`<br>`Buffer.prototype.readDoubleLE`<br>`Buffer.prototype.readDoubleBE`<br>`Buffer.prototype.readBigInt64LE`<br>`Buffer.prototype.readBigInt64BE`<br>`Buffer.prototype.readBigUInt64LE`<br>`Buffer.prototype.readBigUInt64BE` | JS/浏览器 | — | — | 不需要fallback（纯JS Buffer/TypedArray） | 当前路径：JS；Buffer 实例的固定宽度/可变宽度数值读取组；由 browser buffer vendor 提供。具体方法全列出，vendor 内部实现未展开。 来源：`packages/node-compat/src/buffer.js`；`packages/node-compat/src/vendor/index.js` |
| Node API | `Buffer.prototype.writeInt8`<br>`Buffer.prototype.writeUInt8`<br>`Buffer.prototype.writeInt16LE`<br>`Buffer.prototype.writeInt16BE`<br>`Buffer.prototype.writeUInt16LE`<br>`Buffer.prototype.writeUInt16BE`<br>`Buffer.prototype.writeInt32LE`<br>`Buffer.prototype.writeInt32BE`<br>`Buffer.prototype.writeUInt32LE`<br>`Buffer.prototype.writeUInt32BE`<br>`Buffer.prototype.writeIntLE`<br>`Buffer.prototype.writeIntBE`<br>`Buffer.prototype.writeUIntLE`<br>`Buffer.prototype.writeUIntBE`<br>`Buffer.prototype.writeFloatLE`<br>`Buffer.prototype.writeFloatBE`<br>`Buffer.prototype.writeDoubleLE`<br>`Buffer.prototype.writeDoubleBE`<br>`Buffer.prototype.writeBigInt64LE`<br>`Buffer.prototype.writeBigInt64BE`<br>`Buffer.prototype.writeBigUInt64LE`<br>`Buffer.prototype.writeBigUInt64BE` | JS/浏览器 | — | — | 不需要fallback（纯JS Buffer/TypedArray） | 当前路径：JS；Buffer 实例的固定宽度/可变宽度数值写入组；runtime/buffer.js 对数值写入入口补充范围错误码。 来源：`packages/node-compat/src/runtime/buffer.js`；`packages/node-compat/src/vendor/index.js` |
| Node API | `Buffer.prototype.at`<br>`Buffer.prototype.copyWithin`<br>`Buffer.prototype.entries`<br>`Buffer.prototype.every`<br>`Buffer.prototype.filter`<br>`Buffer.prototype.find`<br>`Buffer.prototype.findIndex`<br>`Buffer.prototype.findLast`<br>`Buffer.prototype.findLastIndex`<br>`Buffer.prototype.forEach`<br>`Buffer.prototype.keys`<br>`Buffer.prototype.map`<br>`Buffer.prototype.reduce`<br>`Buffer.prototype.reduceRight`<br>`Buffer.prototype.reverse`<br>`Buffer.prototype.set`<br>`Buffer.prototype.slice`<br>`Buffer.prototype.some`<br>`Buffer.prototype.sort`<br>`Buffer.prototype.subarray`<br>`Buffer.prototype.values` | JS/浏览器 | — | — | 不需要fallback（纯JS Uint8Array继承方法） | 当前路径：JS；Buffer 继承的 TypedArray/Uint8Array 实例方法组；vendor 实例继承关系来自 buffer 包。待核：逐项目标 WebView 语义未在本盘点验证。 来源：`packages/node-compat/src/vendor/index.js`；`packages/node-compat/src/buffer.js` |
| Node API | `Buffer instance .buffer`<br>`Buffer instance .byteLength`<br>`Buffer instance .byteOffset`<br>`Buffer instance .length`<br>`Buffer.prototype.parent`<br>`Buffer.prototype.offset` | JS/浏览器 | — | — | 不需要fallback（纯JS对象属性） | 当前路径：JS；TypedArray/Buffer 数据视图属性和 browser buffer vendor 暴露的 parent/offset 兼容访问器；parent/offset 已属过时兼容属性。 来源：`packages/node-compat/src/vendor/index.js`；`packages/node-compat/src/buffer.js` |
| Node API | `crypto.randomUUID`<br>`crypto.randomBytes`<br>`crypto.createHash`<br>`crypto.createHmac`<br>`crypto.getHashes`<br>`crypto.createSecretKey`<br>`crypto.KeyObject`<br>`crypto.pbkdf2`<br>`crypto.pbkdf2Sync`<br>`crypto.scrypt`<br>`crypto.default (module object)` | JS/浏览器 | — | — | 不需要fallback（纯JS/WebCrypto） | 当前路径：JS；WebCrypto 仅用于随机字节/UUID；hash、HMAC、PBKDF2、scrypt 使用内嵌 @noble/hashes JS 实现。randomBytes callback 由本地 microtask 排队；Buffer 参数/结果没有跨 Bridge。 来源：`packages/node-compat/src/crypto.js`；`packages/node-compat/src/runtime/crypto.js` |
| Node API | `crypto.Hash.update(data,inputEncoding)`<br>`crypto.Hash.digest(encoding)`<br>`crypto.Hmac.update(data,inputEncoding)`<br>`crypto.Hmac.digest(encoding)` | JS/浏览器 | — | — | 不需要fallback（纯JS增量计算） | 当前路径：JS；createHash/createHmac 返回对象的增量方法；实现收敛在 makeIncremental，digest 后重复调用会抛错。 来源：`packages/node-compat/src/runtime/crypto.js` |
| Node API | `crypto.KeyObject.type`<br>`crypto.KeyObject.equals(other)`<br>`crypto.SecretKeyObject.symmetricKeySize`<br>`crypto.SecretKeyObject.export(options)` | JS/浏览器 | — | — | 不需要fallback（纯JS密钥对象） | 当前路径：JS；createSecretKey 返回 SecretKeyObject（实现类未作为命名构造器导出）；密钥材料存于页面内 WeakMap。 来源：`packages/node-compat/src/runtime/crypto.js` |
| Node API | `crypto.timingSafeEqual`<br>`crypto.timingSafeEqual(a,b)` | — | — | 是/按分支 | 不支持（需要Native同步调用，D08禁止IPC同步fallback） | 当前路径：XHR；runtime/crypto.js 调用 callSync('os.timingSafeEqual', ...)；initialize_script.js 的 callSync 使用同步 POST /__niva_sync。不是 WebCrypto，也不走 WS。 来源：`packages/node-compat/src/crypto.js`；`packages/node-compat/src/runtime/crypto.js:288-295` |
| Node API | `zlib.gzip`<br>`zlib.gunzip`<br>`zlib.gzipSync`<br>`zlib.gunzipSync`<br>`zlib.gzip(data,options,callback)`<br>`zlib.gunzip(data,options,callback)`<br>`zlib.gzipSync(data,options)`<br>`zlib.gunzipSync(data,options)` | JS/浏览器 | — | — | 不需要fallback（纯JS压缩；二进制只留在JS内存） | 当前路径：JS；使用 fflate/browser；同步版本同步返回 Buffer，callback 版本在 JS 中完成压缩并排队回调。输出 Buffer 不构成 Native/IPC 数据传输。 来源：`packages/node-compat/src/zlib.js`；`packages/node-compat/src/runtime/zlib.js` |
| Node API | `assert.AssertionError`<br>`assert.fail`<br>`assert.ok`<br>`assert.equal`<br>`assert.notEqual`<br>`assert.strictEqual`<br>`assert.notStrictEqual`<br>`assert.deepEqual`<br>`assert.notDeepEqual`<br>`assert.deepStrictEqual`<br>`assert.notDeepStrictEqual`<br>`assert.throws`<br>`assert.doesNotThrow`<br>`assert.rejects`<br>`assert.doesNotReject`<br>`assert.ifError`<br>`assert.match`<br>`assert.doesNotMatch`<br>`assert.default (module object)` | JS/浏览器 | — | — | 不需要fallback（纯JS断言） | 当前路径：JS；assert ESM 的全部具名 API。 来源：`packages/node-compat/src/assert.js`；`packages/node-compat/src/runtime/assert.js` |
| Node API | `assert.strict`<br>`assert/strict.AssertionError`<br>`assert/strict.fail`<br>`assert/strict.ok`<br>`assert/strict.equal`<br>`assert/strict.notEqual`<br>`assert/strict.strictEqual`<br>`assert/strict.notStrictEqual`<br>`assert/strict.deepEqual`<br>`assert/strict.notDeepEqual`<br>`assert/strict.deepStrictEqual`<br>`assert/strict.notDeepStrictEqual`<br>`assert/strict.throws`<br>`assert/strict.doesNotThrow`<br>`assert/strict.rejects`<br>`assert/strict.doesNotReject`<br>`assert/strict.ifError`<br>`assert/strict.match`<br>`assert/strict.doesNotMatch` | JS/浏览器 | — | — | 不需要fallback（纯JS断言） | 当前路径：JS；strict 属性及 assert/strict 子路径别名；使用本地 AssertionError 和断言函数。 来源：`packages/node-compat/src/assert.js`；`packages/node-compat/src/assert-strict.js` |
| Node API | `stream.Stream`<br>`stream.Readable`<br>`stream.Writable`<br>`stream.Duplex`<br>`stream.Transform`<br>`stream.PassThrough`<br>`stream.default (module object)` | JS/浏览器 | — | — | 不需要fallback（纯JS流对象；不传输Native流） | 当前路径：JS；类构造器重导出自 readable-stream browser 入口；流对象的构造/缓冲/变换留在 JS。底层 Node API 若产出 Native 数据流，需由该 API 自己分类，不能把 stream 类当作 IPC/WS 流传输。 来源：`packages/node-compat/src/stream.js`；`packages/node-compat/src/runtime/stream.js` |
| Node API | `stream.Readable.prototype.read`<br>`stream.Readable.prototype.setEncoding`<br>`stream.Readable.prototype.pause`<br>`stream.Readable.prototype.resume`<br>`stream.Readable.prototype.isPaused`<br>`stream.Readable.prototype.pipe`<br>`stream.Readable.prototype.unpipe`<br>`stream.Readable.prototype.unshift`<br>`stream.Readable.prototype.push`<br>`stream.Readable.prototype.wrap`<br>`stream.Readable.prototype.destroy`<br>`stream.Readable.prototype[Symbol.asyncIterator]` | JS/浏览器 | — | — | 不需要fallback（纯JS流对象；D08禁止IPC流传输） | 当前路径：JS；Readable 实例的公开消费/背压/异步迭代方法组，来自 vendor 类。待核：未逐项运行核对 readable-stream 4.7.0 在目标 WebView 的行为。 来源：`packages/node-compat/src/vendor/index.js:2,16-35`；`packages/node-compat/src/stream.js` |
| Node API | `stream.Writable.prototype.write`<br>`stream.Writable.prototype.setDefaultEncoding`<br>`stream.Writable.prototype.end`<br>`stream.Writable.prototype.cork`<br>`stream.Writable.prototype.uncork`<br>`stream.Writable.prototype.destroy` | JS/浏览器 | — | — | 不需要fallback（纯JS流对象；D08禁止IPC流传输） | 当前路径：JS；Writable 实例写入/结束/缓冲控制方法组，来自 vendor 类。待核：未逐项运行核对 vendor 实例方法语义。 来源：`packages/node-compat/src/vendor/index.js:2,16-35`；`packages/node-compat/src/stream.js` |
| Node API | `stream.Duplex.prototype.read`<br>`stream.Duplex.prototype.write`<br>`stream.Duplex.prototype.end`<br>`stream.Transform.prototype._transform`<br>`stream.Transform.prototype._flush`<br>`stream.PassThrough.prototype._transform` | JS/浏览器 | — | — | 不需要fallback（纯JS流对象；D08禁止IPC流传输） | 当前路径：JS；Duplex 组合 Readable/Writable 表面；Transform 与 PassThrough 提供本地变换钩子。钩子名为可覆写类契约，不表示 Native bridge 流；vendor 细节待核。 来源：`packages/node-compat/src/vendor/index.js:2,16-35`；`packages/node-compat/src/stream.js` |
| Node API | `stream.pipeline`<br>`stream.finished`<br>`stream.addAbortSignal`<br>`stream.compose`<br>`stream.destroy`<br>`stream.from`<br>`stream.fromWeb`<br>`stream.toWeb`<br>`stream.wrap`<br>`stream.isDisturbed`<br>`stream.isErrored`<br>`stream.isReadable` | JS/浏览器 | — | — | 不需要fallback（纯JS流工具；D08禁止IPC流传输） | 当前路径：JS；stream ESM named exports；只组合或检查页面内 stream/web 对象，不自行调用 Native。 来源：`packages/node-compat/src/stream.js`；`packages/node-compat/src/runtime/stream.js` |
| Node API | `stream/promises.pipeline`<br>`stream/promises.finished`<br>`stream.promises.pipeline`<br>`stream.promises.finished`<br>`stream/promises.default (module object)` | JS/浏览器 | — | — | 不需要fallback（纯JS Promise流工具；D08禁止IPC流传输） | 当前路径：JS；stream/promises 子路径的两个具名导出，以及 stream.promises 别名；不把 Promise 包装误记成 Native/IPC 流。 来源：`packages/node-compat/src/stream-promises.js`；`packages/node-compat/src/stream.js` |
| Node API | `string_decoder.StringDecoder`<br>`string_decoder.default (StringDecoder)` | JS/浏览器 | — | — | 不需要fallback（纯JS/TextDecoder） | 当前路径：JS；构造器适配 browser string_decoder，并在 UTF-8 路径使用 TextDecoder；数据仅在页面内解码。 来源：`packages/node-compat/src/string_decoder.js`；`packages/node-compat/src/runtime/string_decoder.js` |
| Node API | `StringDecoder.prototype.write(buffer)`<br>`StringDecoder.prototype.end(buffer)`<br>`StringDecoder instance .encoding`<br>`StringDecoder instance .lastNeed`<br>`StringDecoder instance .lastTotal`<br>`StringDecoder instance .lastChar` | JS/浏览器 | — | — | 不需要fallback（纯JS解码器） | 当前路径：JS；实例方法及状态字段；write/end 由 runtime/string_decoder.js 包装，字段为 vendor StringDecoder 的可见契约。vendor 内部状态细节待核。 来源：`packages/node-compat/src/runtime/string_decoder.js`；`packages/node-compat/src/vendor/index.js:3,44` |
| Node API | `timers.setTimeout`<br>`timers.clearTimeout`<br>`timers.setInterval`<br>`timers.clearInterval`<br>`timers.setImmediate`<br>`timers.clearImmediate`<br>`timers.Timeout`<br>`timers.Immediate`<br>`timers.default (module object)` | JS/浏览器 | — | — | 不需要fallback（浏览器定时器） | 当前路径：JS；ESM named exports；实现调度使用 root.setTimeout/root.setInterval，存在时使用 root.setImmediate，不调用 Native。 来源：`packages/node-compat/src/timers.js`；`packages/node-compat/src/runtime/timers.js` |
| Node API | `timers.Timeout.ref`<br>`timers.Timeout.unref`<br>`timers.Timeout.hasRef`<br>`timers.Timeout.refresh`<br>`timers.Timeout.close`<br>`timers.Timeout[Symbol.toPrimitive]`<br>`timers.Timeout[Symbol.dispose]` | JS/浏览器 | — | — | 不需要fallback（浏览器定时器句柄） | 当前路径：JS；Timeout 句柄公开方法；ref/unref 仅维护本地兼容状态，浏览器生命周期不因此改变。Symbol.dispose 在运行环境提供该符号时挂载。 来源：`packages/node-compat/src/runtime/timers.js` |
| Node API | `timers.Immediate.ref`<br>`timers.Immediate.unref`<br>`timers.Immediate.hasRef`<br>`timers.Immediate[Symbol.toPrimitive]`<br>`timers.Immediate[Symbol.dispose]` | JS/浏览器 | — | — | 不需要fallback（浏览器定时器句柄） | 当前路径：JS；Immediate 句柄公开方法；Symbol.dispose 在运行环境提供该符号时挂载。 来源：`packages/node-compat/src/runtime/timers.js` |
| Node API | `timers/promises.setTimeout`<br>`timers/promises.setImmediate`<br>`timers/promises.setInterval`<br>`timers/promises.scheduler`<br>`timers.promises.setTimeout`<br>`timers.promises.setImmediate`<br>`timers.promises.setInterval`<br>`timers.promises.scheduler`<br>`timers/promises.default (module object)` | JS/浏览器 | — | — | 不需要fallback（浏览器定时器与本地Promise） | 当前路径：JS；Promise 子路径及 timers.promises 别名；setInterval 返回本地 AsyncIterator，scheduler 为本地 wait/yield 对象。 来源：`packages/node-compat/src/timers-promises.js`；`packages/node-compat/src/runtime/timers.js` |
| Node API | `timers.promises.scheduler.wait(delay,options)`<br>`timers.promises.scheduler.yield()`<br>`timers.promises.setInterval iterator.next()`<br>`timers.promises.setInterval iterator.return()`<br>`timers.promises.setInterval iterator.throw(error)`<br>`timers.promises.setInterval iterator[Symbol.asyncIterator]()` | JS/浏览器 | — | — | 不需要fallback（本地Promise/AsyncIterator） | 当前路径：JS；Promise 定时器嵌套方法和 setInterval 迭代器协议；由 runtime/timers.js 的本地定时器调度。 来源：`packages/node-compat/src/runtime/timers.js` |
| Node API | `querystring.parse`<br>`querystring.decode`<br>`querystring.stringify`<br>`querystring.encode`<br>`querystring.escape`<br>`querystring.unescape`<br>`querystring.unescapeBuffer`<br>`querystring.default (module object)` | JS/浏览器 | — | — | 不需要fallback（纯JS字符串编码/解析） | 当前路径：JS；ESM 全部 named exports，decode/encode 分别别名到 parse/stringify；解析、百分号转义及 Buffer 解码均在本地执行。 来源：`packages/node-compat/src/querystring.js`；`packages/node-compat/src/runtime/querystring.js` |
| Node API | `@niva/node-compat.events`<br>`@niva/node-compat.util`<br>`@niva/node-compat.buffer`<br>`@niva/node-compat.crypto`<br>`@niva/node-compat.zlib`<br>`@niva/node-compat.assert`<br>`@niva/node-compat.stream`<br>`@niva/node-compat.string_decoder`<br>`@niva/node-compat.timers`<br>`@niva/node-compat.registerNodeCompat` | JS/浏览器 | — | — | 不需要fallback（ESM模块注册入口） | 当前路径：JS；src/index.js 重导出模块默认对象，并提供 registerNodeCompat；注册同时提供普通模块名与 node: 前缀，stream/promises、timers/promises 和 assert/strict 由 registration.js 单独注册。 来源：`packages/node-compat/src/index.js`；`packages/node-compat/src/runtime/registration.js` |
| Native RPC | `clipboard.read` | — | 异步一次 | — | D08：剪贴板文本列为目标能力；仅文本 unary IPC。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/clipboard.rs:14` |
| Native RPC | `clipboard.write` | — | 异步一次 | — | D08：剪贴板文本列为目标能力；仅文本 unary IPC。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/clipboard.rs:15` |
| Native RPC | `dialog.pickDir` | — | 异步一次 | — | D08：系统对话框列为目标能力；仅异步 unary IPC。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/dialog.rs:17` |
| Native RPC | `dialog.pickDirs` | — | 异步一次 | — | D08：系统对话框列为目标能力；仅异步 unary IPC。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/dialog.rs:18` |
| Native RPC | `dialog.pickFile` | — | 异步一次 | — | D08：系统对话框列为目标能力；仅异步 unary IPC。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/dialog.rs:15` |
| Native RPC | `dialog.pickFiles` | — | 异步一次 | — | D08：系统对话框列为目标能力；仅异步 unary IPC。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/dialog.rs:16` |
| Native RPC | `dialog.saveFile` | — | 异步一次 | — | D08：系统对话框列为目标能力；仅异步 unary IPC。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/dialog.rs:19` |
| Native RPC | `dialog.showMessage` | — | 异步一次 | — | D08：系统对话框列为目标能力；仅异步 unary IPC。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/dialog.rs:14` |
| Native RPC | `extra.focusByWindowId` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达；同名 API 有 macOS 与 Windows 条件注册，均为 unary；注册位置保留两端证据。 来源：`crates/niva/src/app/api/extra.rs:13`；`crates/niva/src/app/api/extra.rs:15` |
| Native RPC | `extra.getActiveWindowId` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/extra.rs:11` |
| Native RPC | `extra.hideApplication` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/extra.rs:19` |
| Native RPC | `extra.hideOtherApplications` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/extra.rs:21` |
| Native RPC | `extra.setActivationPolicy` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/extra.rs:22` |
| Native RPC | `extra.showApplication` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/extra.rs:20` |
| Native RPC | `fs.copy` | — | 异步一次 | 可达 | D08：允许文件复制；当前实现也支持递归目录和 overwrite，限单文件/覆盖策略待定。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/fs.rs:21` |
| Native RPC | `fs.createDir` | — | 异步一次 | 可达 | 待核/默认不开放：文件修改不属于明确的单文件复制例外。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/fs.rs:25` |
| Native RPC | `fs.createDirAll` | — | 异步一次 | 可达 | 待核/默认不开放：文件修改不属于明确的单文件复制例外。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/fs.rs:26` |
| Native RPC | `fs.exists` | — | 异步一次 | 可达 | D08：文件 metadata；允许受限异步 JSON IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/fs.rs:19` |
| Native RPC | `fs.handle` | — | 流式 | — | D08：stream/stateful/raw binary handler 不进 IPC。 | 当前 RPC：拒绝； 来源：`crates/niva/src/app/api/fs.rs:17` |
| Native RPC | `fs.move` | — | 异步一次 | 可达 | 待核/默认不开放：文件修改不属于明确的单文件复制例外。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/fs.rs:23` |
| Native RPC | `fs.node` | — | 异步一次 | 可达 | 部分目标：文本/元数据操作必做，按 op 校验；二进制/同步公开契约不做 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/fs.rs:15` |
| Native RPC | `fs.openHandle` | — | 流式 | — | D08：stream/stateful/raw binary handler 不进 IPC。 | 当前 RPC：拒绝； 来源：`crates/niva/src/app/api/fs.rs:16` |
| Native RPC | `fs.readDir` | — | 异步一次 | 可达 | D08：文件 metadata；允许受限异步 JSON IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/fs.rs:27` |
| Native RPC | `fs.readDirAll` | — | 异步一次 | 可达 | D08：文件 metadata；允许受限异步 JSON IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/fs.rs:28` |
| Native RPC | `fs.readStream` | — | 流式 | — | D08：该 Native stream handler 不进 IPC；文件文本读取是必需能力，但当前只有本地 JS override 经此流读取，IPC 需独立有大小上限的文本 unary。 | 当前 RPC：拒绝；本地 JS override 将 fs.read 映射到此流；无 Native fs.read unary。 来源：`crates/niva/src/app/api/fs.rs:20`；`crates/niva/assets/initialize_script.js:508-510` |
| Native RPC | `fs.remove` | — | 异步一次 | 可达 | 待核/默认不开放：文件修改不属于明确的单文件复制例外。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/fs.rs:24` |
| Native RPC | `fs.stat` | — | 异步一次 | 可达 | D08：文件 metadata；允许受限异步 JSON IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/fs.rs:14` |
| Native RPC | `fs.watch` | — | 流式 | — | D08：stream/stateful/raw binary handler 不进 IPC。 | 当前 RPC：拒绝； 来源：`crates/niva/src/app/api/fs.rs:18` |
| Native RPC | `fs.writeStream` | — | 流式 | — | D08：该 Native stream handler 不进 IPC；文本写入若列必需，需独立有大小上限的文本 unary。 | 当前 RPC：拒绝；本地 JS override 将 fs.write/fs.append 映射到此流；无 Native unary。 来源：`crates/niva/src/app/api/fs.rs:22`；`crates/niva/assets/initialize_script.js:508-535` |
| Native RPC | `host.send` | — | 异步一次 | — | 待核/默认不开放：stdio 父进程消息能力不属于 D08 明确最小集合。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/host.rs:13` |
| Native RPC | `monitor.current` | — | 异步一次 | — | D08：系统/显示 metadata 列为目标能力；仅异步 JSON IPC。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/monitor.rs:14` |
| Native RPC | `monitor.fromPoint` | — | 异步一次 | — | D08：系统/显示 metadata 列为目标能力；仅异步 JSON IPC。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/monitor.rs:16` |
| Native RPC | `monitor.list` | — | 异步一次 | — | D08：系统/显示 metadata 列为目标能力；仅异步 JSON IPC。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/monitor.rs:13` |
| Native RPC | `monitor.primary` | — | 异步一次 | — | D08：系统/显示 metadata 列为目标能力；仅异步 JSON IPC。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/monitor.rs:15` |
| Native RPC | `os.cpus` | — | 异步一次 | 可达 | D08：系统信息/metadata 列为目标能力；仅异步 JSON IPC；不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/os.rs:18` |
| Native RPC | `os.dirs` | — | 异步一次 | 可达 | D08：系统信息/metadata 列为目标能力；仅异步 JSON IPC；不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/os.rs:24` |
| Native RPC | `os.dnsLookup` | — | 异步一次 | 可达 | 待核：DNS/网络信息不是单次 HTTP；需按 D08 网络范围决定。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/os.rs:22` |
| Native RPC | `os.dnsServers` | — | 异步一次 | 可达 | 待核：DNS/网络信息不是单次 HTTP；需按 D08 网络范围决定。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/os.rs:23` |
| Native RPC | `os.eol` | — | 异步一次 | 可达 | D08：系统信息/metadata 列为目标能力；仅异步 JSON IPC；不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/os.rs:26` |
| Native RPC | `os.freemem` | — | 异步一次 | 可达 | D08：系统信息/metadata 列为目标能力；仅异步 JSON IPC；不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/os.rs:19` |
| Native RPC | `os.info` | — | 异步一次 | 可达 | D08：系统信息改由静态 bootstrap metadata 满足，不走 IPC。 | 当前 RPC：授权后可达；当前 Native handler 每次调用 os_info::get()；initialize_script.js 另暴露 Niva.bootstrap。D08 目标是用静态注入满足系统信息。 来源：`crates/niva/src/app/api/os.rs:16`；`crates/niva/assets/initialize_script.js:586` |
| Native RPC | `os.locale` | — | 异步一次 | 可达 | D08：系统信息/metadata 列为目标能力；仅异步 JSON IPC；不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/os.rs:27` |
| Native RPC | `os.networkInterfaces` | — | 异步一次 | 可达 | D08：系统信息/metadata 列为目标能力；仅异步 JSON IPC；不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/os.rs:20` |
| Native RPC | `os.sep` | — | 异步一次 | 可达 | D08：系统信息/metadata 列为目标能力；仅异步 JSON IPC；不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/os.rs:25` |
| Native RPC | `os.timingSafeEqual` | — | 异步一次 | 可达 | 不支持：公开 timingSafeEqual 是同步 Native 且接受二进制；不变成异步 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/os.rs:17` |
| Native RPC | `os.uptime` | — | 异步一次 | 可达 | D08：系统信息/metadata 列为目标能力；仅异步 JSON IPC；不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/os.rs:21` |
| Native RPC | `process.args` | — | 异步一次 | — | D08：进程 metadata 列为目标能力；仅异步 JSON IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/process.rs:20` |
| Native RPC | `process.currentDir` | — | 异步一次 | 可达 | D08：进程 metadata 列为目标能力；仅异步 JSON IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/process.rs:17` |
| Native RPC | `process.currentExe` | — | 异步一次 | — | D08：进程 metadata 列为目标能力；仅异步 JSON IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/process.rs:18` |
| Native RPC | `process.env` | — | 异步一次 | — | 待核：环境变量可能含秘密；不应无筛选地授权远端。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/process.rs:19` |
| Native RPC | `process.execStream` | — | 流式 | — | D08：stream/stateful/raw binary handler 不进 IPC。 | 当前 RPC：拒绝；本地 JS override 将 process.exec 映射到此流；无 Native process.exec unary。 来源：`crates/niva/src/app/api/process.rs:25`；`crates/niva/assets/initialize_script.js:538-555` |
| Native RPC | `process.exit` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/process.rs:22` |
| Native RPC | `process.open` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/process.rs:24` |
| Native RPC | `process.pid` | — | 异步一次 | — | D08：进程 metadata 列为目标能力；仅异步 JSON IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/process.rs:13` |
| Native RPC | `process.setCurrentDir` | — | 异步一次 | 可达 | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/process.rs:21` |
| Native RPC | `process.signal` | — | 流式 | — | D08：stream/stateful/raw binary handler 不进 IPC。 | 当前 RPC：拒绝； 来源：`crates/niva/src/app/api/process.rs:26` |
| Native RPC | `process.spawnSync` | — | 异步一次 | 可达 | D08：同步 Native 子进程操作不进 IPC。 | 当前 RPC：授权后可达；blocking unary；当前 sync_method_allowed 明确包含此名，目标 IPC 不承接同步 Native 契约。 来源：`crates/niva/src/app/api/process.rs:14` |
| Native RPC | `process.stdin` | — | 流式 | — | D08：stream/stateful/raw binary handler 不进 IPC。 | 当前 RPC：拒绝； 来源：`crates/niva/src/app/api/process.rs:16` |
| Native RPC | `process.version` | — | 异步一次 | — | D08：进程 metadata 列为目标能力；仅异步 JSON IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/process.rs:23` |
| Native RPC | `process.write` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/process.rs:15` |
| Native RPC | `resource.exists` | — | 异步一次 | — | D08：文件 metadata；允许受限异步 JSON IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/resource.rs:12` |
| Native RPC | `resource.extract` | — | 异步一次 | — | D08：允许单个内嵌资源复制到目标文件；目标路径与覆盖语义待定。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/resource.rs:13` |
| Native RPC | `resource.readStream` | — | 流式 | — | D08：stream/raw binary 不进 IPC；文本资源读取若列必需，需独立限长的 unary。 | 当前 RPC：拒绝；本地 JS override 将 resource.read 映射到此流；无 Native unary。 来源：`crates/niva/src/app/api/resource.rs:14`；`crates/niva/assets/initialize_script.js:563-570` |
| Native RPC | `shortcut.list` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/shortcut.rs:15` |
| Native RPC | `shortcut.register` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/shortcut.rs:12` |
| Native RPC | `shortcut.unregister` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/shortcut.rs:13` |
| Native RPC | `shortcut.unregisterAll` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/shortcut.rs:14` |
| Native RPC | `socket.control` | — | 流式 | — | D08：stream/stateful/raw binary handler 不进 IPC。 Native 注册表没有一次 HTTP unary；socket.* 不满足该目标。 | 当前 RPC：拒绝； 来源：`crates/niva/src/app/api/socket.rs:83` |
| Native RPC | `socket.tcpAttach` | — | 流式 | — | D08：stream/stateful/raw binary handler 不进 IPC。 Native 注册表没有一次 HTTP unary；socket.* 不满足该目标。 | 当前 RPC：拒绝； 来源：`crates/niva/src/app/api/socket.rs:72` |
| Native RPC | `socket.tcpConnect` | — | 流式 | — | D08：stream/stateful/raw binary handler 不进 IPC。 Native 注册表没有一次 HTTP unary；socket.* 不满足该目标。 | 当前 RPC：拒绝； 来源：`crates/niva/src/app/api/socket.rs:65` |
| Native RPC | `socket.tcpConnectGuarded` | — | 流式 | — | D08：stream/stateful/raw binary handler 不进 IPC。 Native 注册表没有一次 HTTP unary；socket.* 不满足该目标。 | 当前 RPC：拒绝； 来源：`crates/niva/src/app/api/socket.rs:66` |
| Native RPC | `socket.tcpListen` | — | 流式 | — | D08：stream/stateful/raw binary handler 不进 IPC。 Native 注册表没有一次 HTTP unary；socket.* 不满足该目标。 | 当前 RPC：拒绝； 来源：`crates/niva/src/app/api/socket.rs:71` |
| Native RPC | `socket.tlsAttach` | — | 流式 | — | D08：stream/stateful/raw binary handler 不进 IPC。 Native 注册表没有一次 HTTP unary；socket.* 不满足该目标。 | 当前 RPC：拒绝； 来源：`crates/niva/src/app/api/socket.rs:80` |
| Native RPC | `socket.tlsConnect` | — | 流式 | — | D08：stream/stateful/raw binary handler 不进 IPC。 Native 注册表没有一次 HTTP unary；socket.* 不满足该目标。 | 当前 RPC：拒绝； 来源：`crates/niva/src/app/api/socket.rs:73` |
| Native RPC | `socket.tlsConnectGuarded` | — | 流式 | — | D08：stream/stateful/raw binary handler 不进 IPC。 Native 注册表没有一次 HTTP unary；socket.* 不满足该目标。 | 当前 RPC：拒绝； 来源：`crates/niva/src/app/api/socket.rs:74` |
| Native RPC | `socket.tlsListen` | — | 流式 | — | D08：stream/stateful/raw binary handler 不进 IPC。 Native 注册表没有一次 HTTP unary；socket.* 不满足该目标。 | 当前 RPC：拒绝； 来源：`crates/niva/src/app/api/socket.rs:79` |
| Native RPC | `socket.udpBind` | — | 流式 | — | D08：stream/stateful/raw binary handler 不进 IPC。 Native 注册表没有一次 HTTP unary；socket.* 不满足该目标。 | 当前 RPC：拒绝； 来源：`crates/niva/src/app/api/socket.rs:81` |
| Native RPC | `socket.udpSend` | — | 流式 | — | D08：stream/stateful/raw binary handler 不进 IPC。 Native 注册表没有一次 HTTP unary；socket.* 不满足该目标。 | 当前 RPC：拒绝； 来源：`crates/niva/src/app/api/socket.rs:82` |
| Native RPC | `tray.create` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/tray.rs:14` |
| Native RPC | `tray.destroy` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/tray.rs:15` |
| Native RPC | `tray.destroyAll` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/tray.rs:16` |
| Native RPC | `tray.list` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/tray.rs:17` |
| Native RPC | `tray.update` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/tray.rs:18` |
| Native RPC | `webview.baseFileSystemUrl` | — | 异步一次 | — | 现状：ApiManager 硬拒绝；目标仍待核，不因其他 WebView API 授权而开放。 | 当前 RPC：拒绝；当前 IPC 硬拒绝；与 window.open 同为特殊例外。 来源：`crates/niva/src/app/api/webview.rs:15` |
| Native RPC | `webview.baseUrl` | — | 异步一次 | — | 待核：WebView 导航、脚本、cookie 与浏览数据接口需逐项纳入权限表。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/webview.rs:14` |
| Native RPC | `webview.canGoBack` | — | 异步一次 | — | 待核：WebView 导航、脚本、cookie 与浏览数据接口需逐项纳入权限表。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/webview.rs:24` |
| Native RPC | `webview.canGoForward` | — | 异步一次 | — | 待核：WebView 导航、脚本、cookie 与浏览数据接口需逐项纳入权限表。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/webview.rs:25` |
| Native RPC | `webview.clearAllBrowsingData` | — | 异步一次 | — | 待核：WebView 导航、脚本、cookie 与浏览数据接口需逐项纳入权限表。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/webview.rs:30` |
| Native RPC | `webview.closeDevtools` | — | 异步一次 | — | 待核：WebView 导航、脚本、cookie 与浏览数据接口需逐项纳入权限表。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/webview.rs:13` |
| Native RPC | `webview.cookies` | — | 异步一次 | — | 待核：WebView 导航、脚本、cookie 与浏览数据接口需逐项纳入权限表。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/webview.rs:26` |
| Native RPC | `webview.cookiesForUrl` | — | 异步一次 | — | 待核：WebView 导航、脚本、cookie 与浏览数据接口需逐项纳入权限表。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/webview.rs:27` |
| Native RPC | `webview.deleteCookie` | — | 异步一次 | — | 待核：WebView 导航、脚本、cookie 与浏览数据接口需逐项纳入权限表。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/webview.rs:29` |
| Native RPC | `webview.evaluateScript` | — | 异步一次 | — | 待核：WebView 导航、脚本、cookie 与浏览数据接口需逐项纳入权限表。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/webview.rs:16` |
| Native RPC | `webview.goBack` | — | 异步一次 | — | 待核：WebView 导航、脚本、cookie 与浏览数据接口需逐项纳入权限表。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/webview.rs:22` |
| Native RPC | `webview.goForward` | — | 异步一次 | — | 待核：WebView 导航、脚本、cookie 与浏览数据接口需逐项纳入权限表。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/webview.rs:23` |
| Native RPC | `webview.isDevtoolsOpen` | — | 异步一次 | — | 待核：WebView 导航、脚本、cookie 与浏览数据接口需逐项纳入权限表。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/webview.rs:11` |
| Native RPC | `webview.loadHtml` | — | 异步一次 | — | 待核：WebView 导航、脚本、cookie 与浏览数据接口需逐项纳入权限表。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/webview.rs:18` |
| Native RPC | `webview.loadUrl` | — | 异步一次 | — | 待核：WebView 导航、脚本、cookie 与浏览数据接口需逐项纳入权限表。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/webview.rs:17` |
| Native RPC | `webview.openDevtools` | — | 异步一次 | — | 待核：WebView 导航、脚本、cookie 与浏览数据接口需逐项纳入权限表。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/webview.rs:12` |
| Native RPC | `webview.print` | — | 异步一次 | — | 待核：WebView 导航、脚本、cookie 与浏览数据接口需逐项纳入权限表。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/webview.rs:21` |
| Native RPC | `webview.reload` | — | 异步一次 | — | 待核：WebView 导航、脚本、cookie 与浏览数据接口需逐项纳入权限表。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/webview.rs:19` |
| Native RPC | `webview.setCookie` | — | 异步一次 | — | 待核：WebView 导航、脚本、cookie 与浏览数据接口需逐项纳入权限表。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/webview.rs:28` |
| Native RPC | `webview.url` | — | 异步一次 | — | 待核：WebView 导航、脚本、cookie 与浏览数据接口需逐项纳入权限表。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/webview.rs:20` |
| Native RPC | `window.blockCloseRequested` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:157` |
| Native RPC | `window.close` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:95` |
| Native RPC | `window.current` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:93` |
| Native RPC | `window.cursorPosition` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:150` |
| Native RPC | `window.dragResizeWindow` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:113` |
| Native RPC | `window.dragWindow` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:154` |
| Native RPC | `window.fullscreen` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:139` |
| Native RPC | `window.hideMenu` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:99` |
| Native RPC | `window.innerPosition` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:103` |
| Native RPC | `window.innerSize` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:106` |
| Native RPC | `window.isClosable` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:131` |
| Native RPC | `window.isDecorated` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:137` |
| Native RPC | `window.isFocused` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:123` |
| Native RPC | `window.isMaximizable` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:129` |
| Native RPC | `window.isMaximized` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:135` |
| Native RPC | `window.isMenuVisible` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:101` |
| Native RPC | `window.isMinimizable` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:127` |
| Native RPC | `window.isMinimized` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:133` |
| Native RPC | `window.isResizable` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:125` |
| Native RPC | `window.isVisible` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:121` |
| Native RPC | `window.list` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:96` |
| Native RPC | `window.open` | — | 异步一次 | — | 现状：ApiManager 对此名硬拒绝；D08 窗口操作范围需单独决定是否保留例外。 | 当前 RPC：拒绝；列在 D08 窗口操作目标中，但当前 IPC 硬拒绝；请单独确认是否维持此例外。 来源：`crates/niva/src/app/api/window.rs:94` |
| Native RPC | `window.outerPosition` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:104` |
| Native RPC | `window.outerSize` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:108` |
| Native RPC | `window.requestRedraw` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:115` |
| Native RPC | `window.requestUserAttention` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:143` |
| Native RPC | `window.scaleFactor` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:102` |
| Native RPC | `window.sendMessage` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:97` |
| Native RPC | `window.setAlwaysOnBottom` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:142` |
| Native RPC | `window.setAlwaysOnTop` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:141` |
| Native RPC | `window.setBackgroundColor` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:117` |
| Native RPC | `window.setClosable` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:132` |
| Native RPC | `window.setContentProtection` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:144` |
| Native RPC | `window.setCursorGrab` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:152` |
| Native RPC | `window.setCursorIcon` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:149` |
| Native RPC | `window.setCursorPosition` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:151` |
| Native RPC | `window.setCursorVisible` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:153` |
| Native RPC | `window.setDecorated` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:138` |
| Native RPC | `window.setFocus` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:124` |
| Native RPC | `window.setFocusable` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:118` |
| Native RPC | `window.setFullscreen` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:140` |
| Native RPC | `window.setIgnoreCursorEvents` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:155` |
| Native RPC | `window.setImePosition` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:116` |
| Native RPC | `window.setInnerSize` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:107` |
| Native RPC | `window.setMaxInnerSize` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:110` |
| Native RPC | `window.setMaximizable` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:130` |
| Native RPC | `window.setMaximized` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:136` |
| Native RPC | `window.setMenu` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:98` |
| Native RPC | `window.setMinInnerSize` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:109` |
| Native RPC | `window.setMinimizable` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:128` |
| Native RPC | `window.setMinimized` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:134` |
| Native RPC | `window.setOuterPosition` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:105` |
| Native RPC | `window.setProgressBar` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:114` |
| Native RPC | `window.setResizable` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:126` |
| Native RPC | `window.setTheme` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:112` |
| Native RPC | `window.setTitle` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:119` |
| Native RPC | `window.setVisible` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:122` |
| Native RPC | `window.setVisibleOnAllWorkspaces` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:145` |
| Native RPC | `window.setWindowIcon` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:111` |
| Native RPC | `window.showMenu` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:100` |
| Native RPC | `window.theme` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:156` |
| Native RPC | `window.title` | — | 异步一次 | — | D08：窗口操作列为目标能力；仅异步 unary IPC，不承接同步调用契约。 | 当前 RPC：授权后可达； 来源：`crates/niva/src/app/api/window.rs:120` |
| Native RPC | `windowExtra.allowsAutomaticWindowTabbing` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达；平台条件注册；仅对应 target cfg 存在。 来源：`crates/niva/src/app/api/window_extra.rs:60` |
| Native RPC | `windowExtra.beginResizeDrag` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达；平台条件注册；仅对应 target cfg 存在。 来源：`crates/niva/src/app/api/window_extra.rs:40` |
| Native RPC | `windowExtra.hasShadow` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达；平台条件注册；仅对应 target cfg 存在。 来源：`crates/niva/src/app/api/window_extra.rs:52` |
| Native RPC | `windowExtra.hasUndecoratedShadow` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达；平台条件注册；仅对应 target cfg 存在。 来源：`crates/niva/src/app/api/window_extra.rs:45` |
| Native RPC | `windowExtra.isDocumentEdited` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达；平台条件注册；仅对应 target cfg 存在。 来源：`crates/niva/src/app/api/window_extra.rs:55` |
| Native RPC | `windowExtra.resetDeadKeys` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达；平台条件注册；仅对应 target cfg 存在。 来源：`crates/niva/src/app/api/window_extra.rs:39` |
| Native RPC | `windowExtra.setActivationPolicyAtRuntime` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达；平台条件注册；仅对应 target cfg 存在。 来源：`crates/niva/src/app/api/window_extra.rs:67` |
| Native RPC | `windowExtra.setAllowsAutomaticWindowTabbing` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达；平台条件注册；仅对应 target cfg 存在。 来源：`crates/niva/src/app/api/window_extra.rs:56` |
| Native RPC | `windowExtra.setBadgeLabel` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达；平台条件注册；仅对应 target cfg 存在。 来源：`crates/niva/src/app/api/window_extra.rs:72` |
| Native RPC | `windowExtra.setDockVisibility` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达；平台条件注册；仅对应 target cfg 存在。 来源：`crates/niva/src/app/api/window_extra.rs:71` |
| Native RPC | `windowExtra.setEnable` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达；平台条件注册；仅对应 target cfg 存在。 来源：`crates/niva/src/app/api/window_extra.rs:36` |
| Native RPC | `windowExtra.setHasShadow` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达；平台条件注册；仅对应 target cfg 存在。 来源：`crates/niva/src/app/api/window_extra.rs:53` |
| Native RPC | `windowExtra.setIsDocumentEdited` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达；平台条件注册；仅对应 target cfg 存在。 来源：`crates/niva/src/app/api/window_extra.rs:54` |
| Native RPC | `windowExtra.setOverlayIcon` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达；平台条件注册；仅对应 target cfg 存在。 来源：`crates/niva/src/app/api/window_extra.rs:43` |
| Native RPC | `windowExtra.setRtl` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达；平台条件注册；仅对应 target cfg 存在。 来源：`crates/niva/src/app/api/window_extra.rs:44` |
| Native RPC | `windowExtra.setSimpleFullscreen` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达；平台条件注册；仅对应 target cfg 存在。 来源：`crates/niva/src/app/api/window_extra.rs:51` |
| Native RPC | `windowExtra.setSkipTaskbar` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达；平台条件注册；仅对应 target cfg 存在。 来源：`crates/niva/src/app/api/window_extra.rs:41` |
| Native RPC | `windowExtra.setTabbingIdentifier` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达；平台条件注册；仅对应 target cfg 存在。 来源：`crates/niva/src/app/api/window_extra.rs:64` |
| Native RPC | `windowExtra.setTaskbarIcon` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达；平台条件注册；仅对应 target cfg 存在。 来源：`crates/niva/src/app/api/window_extra.rs:37` |
| Native RPC | `windowExtra.setTrafficLightInset` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达；平台条件注册；仅对应 target cfg 存在。 来源：`crates/niva/src/app/api/window_extra.rs:66` |
| Native RPC | `windowExtra.setUndecoratedShadow` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达；平台条件注册；仅对应 target cfg 存在。 来源：`crates/niva/src/app/api/window_extra.rs:42` |
| Native RPC | `windowExtra.simpleFullscreen` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达；平台条件注册；仅对应 target cfg 存在。 来源：`crates/niva/src/app/api/window_extra.rs:50` |
| Native RPC | `windowExtra.tabbingIdentifier` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达；平台条件注册；仅对应 target cfg 存在。 来源：`crates/niva/src/app/api/window_extra.rs:65` |
| Native RPC | `windowExtra.theme` | — | 异步一次 | — | 待核/默认不开放：不在已明确的文件文本、metadata、单次 HTTP、复制、剪贴板、窗口或对话框目标中。 | 当前 RPC：授权后可达；平台条件注册；仅对应 target cfg 存在。 来源：`crates/niva/src/app/api/window_extra.rs:38` |

表格核查：Native 注册名与源码字面注册全集及公开类型交叉对照；旧 Node 179 项按模块逐项映射到当前表，不能把旧状态或本表入口存在性等同于兼容测试通过。当前源码的传输路线不代表目标 API 整合、静态化或 fallback 已实现。本轮只读核查与写入 review 档案，未运行应用或测试。

本轮覆盖核对结果：189 个唯一 Native 注册名（190 条注册语句，1 个平台条件分支重名）；164 个类型声明方法均能对应到 Native 注册或 5 个 JS override；旧 Node 清单 179 项全部映射。189 Native 行、221 Node 行/组、27 顶层/补充行合计 437 行/组。这是源码路由盘点，不是平台运行或 Node 一致性验收。

现状补充：当前 Native IPC 仅接受 unary，拒绝本地服务 origin，需精确来源及方法授权；当前单请求/响应上限 256 KiB。window.open 与 webview.baseFileSystemUrl 硬拒绝；后续是否改变这两个例外仍待决策。表内同步 XHR 的 Native RPC 可达性不代表其公开 Node 接口可以改变同步/异步契约。

本轮分工：luna_deep_worker /root/architecture_inventory 提供 Native 注册全集；luna_deep_worker /root/startup_boundaries 提供文件/进程/系统/网络模块；luna_deep_worker /root/api_js_inventory 提供纯 JS 模块与 crypto 例外。主线程核对顶层入口、补充 JS override 和目标入口、执行清单交叉核对并生成统一表。仅本 review 档案发生本任务的仓库写入；临时清单位于 /tmp，未运行测试或应用。

### R05 / 第 1 轮：窗口身份、来源与方法授权

日期：2026-09-25。用户要求保留 API 分类表，IPC fallback 具体清单稍后再确认，然后继续 review。本轮回到权限讲解，不额外冻结 API 范围。

本轮由主线程复用已盘点的三种通道，并针对性补读权限配置、同步 XHR 和平台 IPC 来源获取；没有重新委派重复盘点，没有运行测试或修改源码。状态：讲解中，待用户讨论。

核心区分：页面能否加载、能否连通 Bridge、某个 API 是否受支持、调用是否获授权是不同判断。Niva 对象存在、CSP 允许网络连接或方法名出现在类型里，都不能代替 Rust 授权。

| 通道/对象 | 身份与来源 | 当前 Rust 检查 | 能力限制 |
|---|---|---|---|
| WS | 窗口启动随机 token + 浏览器 Origin，hello 声明窗口ID再核对 | token 找窗口、精确 trusted origin、Host/路径、hello协议版本/窗口ID | 受信本地上下文可使用已注册普通/流式方法；连接内任务独立路由 |
| 同步 XHR | POST正文窗口token + Origin | token 找窗口、精确 trusted origin、Host、同步方法范围及 unary handler | 同步Native调用；并不因此允许UI主线程/流式方法 |
| 平台 IPC | 原生绑定窗口ID、平台消息的来源URL | 每次调用按该窗口的精确HTTP(S) origin及方法配置检查；额外硬拒绝项 | 一次性异步JSON；fallback目标清单待二次确认 |
| JS/静态信息 | 页面内实现或已经注入的数据 | 原生数据须在注入时守住来源/窗口边界 | 无Native调用的读取不再触发一次Bridge校验，不意味任意页面可获得敏感快照 |

本地受信页面与外源的区别：当前 builder 只向满足受信条件的顶层页注入窗口凭据；同源 iframe 可继承凭据并独立连WS，跨源 iframe不能因此自动继承权限。外源页面默认没有方法授权，经当前 window.permissions 明确授予后才可调用对应IPC方法。

当前配置模型：`window.permissions` 的键为规范化的精确 origin（协议、主机、端口），值为方法名集合，可写具体方法或 namespace.*。同一 origin 的不同路径不构成权限隔离，配置键不能包含页面路径；不同窗口有自己的权限表。比如某窗口为 `https://example.test:8443` 授权 clipboard.read，不代表另一窗口也授权，也不代表同站其他端口或其他方法获权。namespace.* 会覆盖该命名空间新增方法的匹配，不能将它解释为冻结的方法清单。

来源获取：macOS IPC 从 WKScriptMessage.frameInfo().request() 取得来源；Windows iframe IPC 从平台消息 Source 取得来源，回复发送回对应 frame。窗口身份来自原生绑定，不采信JSON里自报的URL/窗口号。当前来源校验用于判断方法授权；frame导航、等待中的响应与跨窗口参数等完整边界仍需后续专项核查，不能据此宣称全部平台安全验收。

授权粒度的当前限制：WindowPermissions 按origin和方法授权，不是文件路径、网络目标或方法参数的细粒度沙箱。允许某个fs方法不等于仅允许访问项目资源根；ResourceManager 的根目录限制是另一层资源规则。后续文件/HTTP fallback要在最终API列表确认时一并确认参数和资源权限，尤其 fs.node 这类多操作入口不能只看外层方法名。

与前述设计的衔接：D04 的 --config/--resource 只是输入来源接口，不能自动授予debug权限；D07若代理开发页面到应用origin，相当于把明确的开发源放进可信上下文，需明确其授权范围；D08 fallback只更换获准的传输实现，不扩大权限或改变公开API契约。

源码依据：`window_manager/permissions.rs::WindowPermissions::new/allows`；`window_manager/builder.rs::build_webview`；`http_server/mod.rs::ws_pump_inner/handle_sync`；`api_manager/mod.rs::ipc_call/sync_call`；`ipc_macos.rs::handle_message`；`ipc_windows_frames.rs` 的 Source 与 frame 回复路径。本轮未新增缺陷结论，方法与资源参数权限作为后续R05讨论项。

### R05 确认与 R06 开始 — 2026-09-25

用户表示“没问题，继续”，确认 R05 本轮权限模型讲解；未覆盖的导航/iframe/参数权限专项仍保留，IPC fallback 逐项范围尚未冻结。当前推进 R06 并发、状态与资源所有权。

### R06 / 第 1 轮：主线程、异步任务与阻塞工作

本轮主线程串联 JS → WS → API调度 → Native 的执行路径；Luna 只读核查 main_exec、blocking 与调度实现。仅更新本档，不运行测试或修改代码。

| 执行位置 | 实际职责 | 需要掌握的边界 |
|---|---|---|
| tao 主线程 | 原生事件循环、需主线程执行的窗口/菜单等操作 | 不能等待必须由自己处理的回调完成 |
| HTTP 接收驱动 niva-http | loopback listener 与连接任务派发 | 接收和异步等待不应被长时间同步操作阻塞 |
| API 接收驱动 niva-api | 从有界队列取出WS调用，再 spawn 每个 run_job | 驱动线程不等于每个API都只在该线程顺序执行；实际 handler 由 smol task 执行 |
| smol 异步任务 | 等待I/O、任务结果、取消与超时等 | async 本身不能将阻塞系统调用自动变成非阻塞 |
| smol::unblock 阻塞工作池 | blocking! 包装的同步I/O/系统调用；当前每个WS连接的阻塞消息泵也在此运行 | 等待者取消或超时，不保证已经开始的闭包立即停止 |
| 自定义协议 worker | 第三章的资源读取/页面加工 | 独立调度限制，不应与API派发队列混为一个池 |

正常窗口操作示例：页面异步调用 window.setTitle → WS 收到请求 → API任务查找窗口 → run_on_main 将闭包经事件循环 proxy 投递到 tao → 主线程执行 → 单次结果通道唤醒API任务 → WS返回 → 页面Promise结束。窗口操作不是把 UI 对象任意交给后台线程调用。

普通阻塞I/O：register_blocking_api/显式 blocking! 把工作交给 smol::unblock，外层 async handler 等待结果。API文件中函数写成 async，并不证明内部每个操作都已正确分离阻塞行为；更具体Native模块在R08核查。

共享状态：Arc 允许多处持有同一对象并维持生命期，不等于线程安全或循环引用自动回收；Arc<Mutex<T>> 保护可变状态。注册完成的handler映射使用不可变读取；窗口映射、连接表、活跃调用等有独立锁。设计要求是短时间拿锁、取出需要的对象后释放，不跨 await 持有普通Mutex，也不拿着WindowManager锁再嵌套清理其他管理器。当前源码对多个包装类型使用 unsafe Send/Sync 声明，仍需遵守实际平台线程约束，不能把该声明当作平台安全证明。

容量与生命周期：WS派发队列默认容量64，取出后即spawn独立任务，因此队列容量不等于总在途任务上限；WS发送端当前使用std mpsc队列，具体流量控制在AR-011及后续压力验收检查。普通WS任务默认执行超时30秒，方法可覆盖；IPC另有并发计数；同步XHR路径刻意等待明确结果，不沿用WS timeout语义。不同通道必须分别记录限制，不统一宣称“都被30秒保证终止”。

同步XHR的特殊性：页面线程在调用期间被阻塞，Native 若再等待该页面所属UI线程执行，就有互相等待风险。因此当前同步路由排除了UI/流式调用；不能为接口统一而把窗口操作塞进同步调用。

取消与超时的语义：请求等待终止、Native操作停止、系统资源释放、已发生副作用回滚是四件事。当前WS以handler future与计时器/取消信号竞速；已开始的blocking闭包、已经投递的主线程操作是否继续执行，需要单独处理。超时只告诉调用方未得到确定结果，不能据此自动重试有副作用操作。

本轮源码：api_manager/mod.rs::new/dispatch/run_job/register_blocking_api/sync_call；utils.rs::blocking/ArcMut；main_exec.rs::run_on_main；http_server/mod.rs::start/accept_loop/ws_pump_inner；api/window.rs 的 run_on_main 调用。状态：讲解中，等待用户讨论；已有AR-005、AR-009至AR-012继续关联本章。

R06 Luna 核查补充：`api_manager/thread_pool.rs` 的旧固定worker池当前未被模块声明或调用，不是活动API执行路径；应避免按文件名将其描述为当前线程池。当前 blocking! 实际使用 smol::unblock。`process.exit` 经 run_on_main 设置 ExitWithCode。上述为源码核查，非线程行为实测。

#### AR-013：派发队列拒绝后残留活跃调用状态

- 归属：R06；类别：缺陷；状态：已确认待决策；优先级：P2。
- 证据：`api_manager/mod.rs::dispatch` 约641–677行先active.insert，随后dispatch_tx.try_send失败时只返回busy（-3），没有移除相应active记录。
- 影响：被拒绝且未执行的任务仍留在活跃表，可能直到取消、断连或关窗才清除。未运行负载复现，不宣称已出现可测泄漏规模。
- 候选方向与验收：入队失败完整撤销该次注册；重复拒绝不增长活跃表，不误删其他任务，结合AR-012处理请求身份。

#### AR-014：主线程回调可能在调用超时后仍产生副作用

- 归属：R06/R07；类别：生命周期与错误契约；状态：已确认待决策；优先级：P2。
- 证据：`main_exec.rs::run_on_main` 把闭包投递到事件循环后，外层等待结果；回调执行的是 tx.try_send(f(...))，即先执行f，即使接收者已因handler超时被丢弃也会执行。
- 影响：调用方先收到超时，已排队的窗口/其他操作随后仍可能发生。静态控制流结论，未模拟卡顿验证。
- 候选方向：明确未开始的主线程任务能否取消、执行中的操作如何报告结果、超时是否为结果未知；避免调用方误认为超时等于未执行。不得自动重试有副作用操作。
- 未来验收：主线程延迟、调用超时/取消、窗口关闭与最终回调交错时，终态和副作用有明确契约。

本轮 `luna_deep_worker`（/root/startup_boundaries）交付执行器与取消边界核查；主线程核对正常网络/UI流程、解释锁与所有权并整合AR-013/014。没有修改实现或运行测试。

### R07 / 第 1 轮：窗口与桌面交互

日期：2026-09-25。用户要求继续，当前从 R06 推进 R07；R06 的并发批注保留待处理。状态：讲解中。主线程核查窗口/WebView/事件入口，Luna 只读核查菜单、托盘与快捷键。未修改代码或运行真机验证。

1. **一个窗口由哪些对象组成**：NivaWindow 保存 tao 原生窗口、Wry WebView、Niva窗口ID、系统WindowId、窗口token、来源/权限、菜单句柄、连接表和关闭拦截状态。WindowManager维护这些对象的映射；原生UI对象与网页文档是两个层次。
2. **创建与配置**：window.open在主线程按传入NivaWindowOptions创建窗口。默认空选项不是复制主窗口全部配置；例如新窗口不自动继承主窗口permissions。标题等部分字段会回退应用级默认值，具体由builder处理。原生parent/owner是平台窗口关系，不等于Rust管理器已实现递归子窗口生命周期。
3. **窗口身份与存储**：窗口token/连接/方法权限独立；WindowManager持有共享WebContext及应用data目录，不能把不同窗口视为独立浏览器profile或独立存储沙箱。Niva窗口ID为进程内u8标识，不是持久应用UUID。
4. **跨窗口操作与事件**：大量窗口API接可选目标ID，省略时操作调用窗口；window.sendMessage发往目标窗口。窗口级事件通过目标窗口的所有已连接WS广播，同窗同源iframe也可收到；单次API结果则只回原请求连接。源码名send_ipc_event实际调用WS发送，不表示IPC fallback已经有事件推送。
5. **关闭、隐藏、后台**：隐藏只改变可见性，窗口/WebView/事件循环继续存在；关闭拦截只影响用户CloseRequested路径，收到window.closeRequested后由页面决定保存、隐藏或显式关闭。主窗口关闭终止应用；有托盘不意味着主窗口可被销毁后自动常驻。API close与OS关闭的清理差异仍按AR-005修复。
6. **平台桌面行为**：macOS菜单挂在应用菜单栏，并随焦点窗口切换；Windows菜单挂到具体窗口。macOS parent与Windows parent/owner分别映射各自原生字段，不承诺行为完全等价。原生句柄应按主线程约束使用，窗口参数支持不等于两个平台均已验收。
7. **WebView自身行为**：Niva的window.open是明确的原生窗口API。当前浏览器弹窗请求（target=_blank等）会被拒绝并报告webview.newWindowRequested；浏览器直接下载也被拒绝并报告事件，不等于未来D08的Native文件下载能力已实现。webview.loaded表示WebView加载完成回调，不是业务ready或HTTP成功保证。
8. **其他桌面API**：对话框以调用窗口为父窗口，当前经run_on_main执行；剪贴板当前提供文本读写。显示器API返回逻辑/物理坐标及scaleFactor，窗口布局应区分坐标单位。运行细节及错误语义在R08逐API复核。

典型托盘应用的现有用法：保留主窗口，拦截用户关闭请求并隐藏；托盘或快捷键事件让它重新显示；显式“退出”再结束应用。这里只说明如何组合当前机制，不自动增设close-to-tray配置或改变默认关闭政策。IPC页面的桌面事件能力须与后续fallback清单一起决定，不能因某个控制API返回成功就假设持续事件同样可用。

源码：api/window.rs::open/close/send_message/block_close_requested；window_manager/mod.rs::WindowManager；window_manager/window.rs::NivaWindow/send_ipc_event；window_manager/builder.rs::build_window/build_webview；event_handler.rs::handle_window_event；api/dialog.rs、clipboard.rs、monitor.rs。用户确认待记录。

#### AR-015：进程内 ID 分配耗尽边界需修正

- 归属：R07；类别：缺陷；状态：已确认待决策；优先级：P2。
- 证据：`utils.rs::IdCounter::next` 使用u8 next_id及普通 +=1，循环次数为0..u8::MAX；边界处加法可溢出，单次扫描也不是完整256个值。
- 影响：ID空间接近耗尽/复用时可能在debug溢出panic或出现未检查完整空间即返回失败；静态代码结论，未运行大量窗口/资源创建测试。
- 候选方向：明确最大活跃数量与ID复用，采用可控的边界处理，耗尽时返回普通错误；确认对所有IdCounter使用者的一致规则。
- 未来验收：接近上限、全部占用、释放后重用、重复创建/销毁均不panic、不重复分配活跃ID；不把瞬时ID当持久身份。

R07 菜单/托盘/快捷键补充：

- 托盘ID在manager内分配，每项保存owner窗口；更新/销毁API检查owner。配置或调用创建的托盘归相应窗口所有，关闭清理按owner批量移除。
- 快捷键内部把系统HotKey ID映射到窗口ID与action ID；action ID按窗口分配。重复全局组合键或重复窗口/action ID报错，不能把多窗口快捷键看作彼此完全独立的系统注册空间。
- tray和shortcut API操作均经run_on_main；初始化全局快捷键管理器失败后保存错误，后续注册返回错误，不在此处直接panic。
- 菜单项ID包含窗口ID和item ID；muda菜单事件由EventHandler解析后交给对应窗口，托盘/快捷键事件也先反查owner，再发窗口级事件。托盘title当前只在macOS设置，其他平台忽略。
- Luna确认两个manager提供owner维度清理；主线程此前已核对WindowManager::cleanup_window调用它们。清理途中失败是否阻断剩余步骤仍属于AR-005，不把有清理函数等同于完整生命周期验收。

本轮luna_deep_worker（/root/architecture_inventory）交付归属、ID、主线程与平台差异盘点；主线程整合窗口/WebView行为与已有关闭证据。没有修改源码或运行真机验证。

### R08 / 第 1 轮：文件、网络、进程与错误契约

日期：2026-09-25。用户要求继续；R07 已讲解，批注留待收口。R08 复用API分类总表，不重复枚举。IPC fallback最终范围仍待二次确认。

分层职责：页面API负责Node风格参数、callback/Promise/对象和编码等语义；Bridge搬运请求/结果；Rust提供真实文件、socket、子进程和系统操作。相同公开行为可能复用同一原生入口，不能从Rust方法数量推断公开API兼容程度。

**文件路径与返回形态**：当前Node风格普通文件操作通过 `fs.node`，以操作名和参数选择readFile/writeFile/stat等；异步入口调用WS unary，同步入口调用sync XHR。当前readFile原始字节经Base64放在JSON中返回，再由JS按encoding转换为字符串或Buffer，Stats等对象的方法也由JS补回。这里描述当前代码，不表示D08已经批准IPC二进制转运：文本fallback要有明确文本契约，不能因现行JSON里装得下Base64就算通过范围确认。旧Niva.api.fs.read/write等JS override另走readStream/writeStream，是API总表中刻意区分的旧入口。

**网络分层**：当前Node http/https客户端和服务端在JS中处理HTTP/1.1请求、响应与流对象，下面复用net/tls及Rust socket。Native socket注册表管理TCP/TLS/UDP连接和listener，Owner是窗口ID+连接ID；取句柄时核对Owner，不允许换一个frame连接接管旧handle。长连接、接收后待attach的连接及读流量有各自限制/清理逻辑，不等于一个JSON请求。

DNS也分两条：lookup走系统解析接口os.dnsLookup；resolve系列在JS组织DNS协议，使用UDP/TCP传输。不能把所有DNS都描述为一次系统调用，也不能直接推断其IPC范围。

当前HTTP使用guarded连接，目标地址检查在Rust端执行：允许公网，另有Niva自身loopback地址及同owner本地listener例外；不是任意内网/localhost都可访问。原生TCP入口与guarded HTTP的目标策略不同，是否符合应用实际联网需求留本章讨论，不直接认定为缺陷。

**进程分层**：Node process表示当前应用进程，child_process表示新建子进程。真实cwd是进程级共享状态；process.chdir会影响同一Niva进程的相对路径解析，不是窗口私有目录。子进程可单独设置currentDir和env，当前Rust ExecOptions提供env时会先env_clear再设置给定环境，不是对用户全局环境变量作持久修改。

普通execStream把stdin/stdout/stderr连接到流式调用，并按窗口/连接/请求登记子进程；专门的supervisor线程等待退出并观察取消，取消时kill并wait回收。这里仅核对直接子进程的这条路径，不把它推广成所有后代进程树必然被终止。detached明确退出这套等待/取消归属，立即返回PID；是否为OS会话意义的完整daemon化须另核，不能由名称推断。同步spawnSync有独立实现与选项，不套用异步流式的超时结论。

**错误的三层**：系统错误（ENOENT/EACCES/ECONNREFUSED等）、Bridge错误（权限、超时、繁忙、断线等）、JS参数/契约错误。Rust native_error_data把部分std::io::Error映射成code/errno；JS runtime.nativeError构造Error并保留bridgeCode/bridgeResult，部分路径还根据message推断错误。未来统一runtime应保持同一方法在允许的传输中有一致错误契约，不能只保证有一个message或Promise reject。

典型失败例子：异步读不存在文件，应以文件不存在语义失败；句柄所属连接消失，应关闭或拒绝继续使用，不能重连后按旧handle继续；进程超时/取消的返回与实际退出是不同验收点。Native元数据/原始字节成功返回不等于Node对象全部语义已经兼容，R10继续核查。

后续讨论项：① fs.node这类复用入口的权限是否要按具体操作细分；② HTTP内网目标策略；③ 进程级cwd与多窗口共享；④ 文件/网络fallback的大小、取消及错误契约。此轮不据此新增实现授权或冻结选项。

源码：`packages/node-compat/src/runtime/fs.js::invoke/decode`、`http.js::ClientRequest`、`dns.js::lookup`、`bridge.js::nativeError`；`crates/niva/src/app/api/socket.rs::Owner/active_handle/guarded_address_allowed`；`api/process.rs::build_exec_command/exec_stream/wait_for_child/set_current_directory`；`api_manager/mod.rs::native_error_data`。静态核查，无本轮运行测试。

R08 文件生命周期补充（Luna静态核查）：

- fs.node是一次blocking unary，每次按op执行相应文件操作；二进制边界用JSON Base64，Rust也承担边界编解码，JS负责Node编码/Buffer/Stats等公开语义。不得误述为只有JS才处理编码。
- fs.readStream/writeStream按路径操作，没有跨调用handle ID；读取返回分块与最终size，写入收到END后返回bytes。fs_ops中的copy/move/remove为独立文件系统操作，文件内容不经过页面。
- fs.openHandle打开并持有文件，生成随机handle ID，发open事件并维持流；表当前上限256。Owner为窗口ID+连接ID（不是请求ID），后续fs.handle按Owner检查，单文件Mutex串行操作。这里的read/write是JSON Base64操作，不是readStream/writeStream那种原始分块。
- 显式close从注册表移除句柄并通知open流退出；请求取消时通过RAII清理注册项。fs.watch是单独持续事件流，退出后释放watcher，不占用文件handle表。
- 状态正确释放仍不等于已经执行的文件修改被撤销；延续R06的取消语义边界。

本轮luna_deep_worker（/root/startup_boundaries）交付文件操作/句柄生命周期核查，未报告额外新问题；主线程负责网络、进程、错误与章节整合。仅静态核查及review档案写入，没有修改源码或运行测试。

### D10：统一资源对象生命周期

日期：2026-09-25。用户提出资源对象基类，在对象销毁时自动调用close/释放，以降低泄露概率。方向记录为待实现；用户所说dispatch按dispose（释放）理解，若另有所指再修订。

- 统一管理资源身份、Owner、open/closing/closed状态、幂等close/dispose、关闭后调用拒绝和监听器注销。可用资源基类或内部资源持有组件承载；已有Node Stream/EventEmitter继承关系应复用内部组件，不能为统一基类破坏公开对象契约。
- JS层显式close/destroy/dispose是主要路径；需等待原生完成的关闭应提供可等待的异步完成语义。Symbol.dispose适合同步释放，Symbol.asyncDispose/await using可表达异步作用域释放，实际WebView/构建支持要核查，不能把当前引擎支持视为已验收。
- FinalizationRegistry可作“对象失去引用后尝试清理”的兜底；垃圾回收时间和回调是否执行均不保证，页面/进程退出时也不能依赖它。finalizer只保留必要handle/owner信息，不得反向强引用被追踪对象；监听器/回调长期持有对象时，GC兜底不会触发。
- Rust继续承担确定性Owner清理：显式关闭、连接结束、窗口销毁时撤销对应资源，RAII/Drop按资源语义释放。即使JS finalizer无法发消息，也不能因此失去Native兜底。detached进程按其明确例外处理；强制进程终止不保证所有用户级清理逻辑运行。
- 释放函数要幂等，应对显式关闭、GC兜底和连接清理相遇；旧连接finalizer不能经新连接误关其他资源。资源释放不改变AR-005/009/014中待统一的终态和副作用契约。
- 验收：显式关闭/重复关闭、异常退出作用域、页面刷新、frame移除、断线、窗口销毁、回收迟到均无重复关闭误伤和遗留资源。不能用等待GC作为必要正确性测试的唯一手段。

资料依据：MDN FinalizationRegistry明确不保证回调发生/时机，https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/FinalizationRegistry ；异步释放协议见 https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/await_using 。本轮查阅用于说明机制边界，未编写或执行资源基类。

### D11：HTTP/HTTPS 允许内网和外网

日期：2026-09-25。用户明确要求像Node程序一样访问内网和外网。状态：已决策待实现。

- 获准使用Native网络API的页面，其HTTP/HTTPS目标可为公网、私网、localhost/loopback和局域网域名；不再仅因解析到非公网地址而整体拒绝。
- 当前http/https使用connectGuarded以及Rust guarded_address_allowed的公网/有限本地例外策略，与目标存在差异；后续应在相关连接/地址解析路径统一调整，不仅在JS改判断。原有章节和API表描述的是当前实现，以本条作为新的目标策略。
- 这改变目标地址范围，不改变页面/窗口的API授权、TLS证书验证和请求错误契约。页面仍不能因为换IPC传输而获得未授予的方法权限。
- 一次性HTTP IPC fallback若最终纳入，也应遵循相同地址策略；fallback具体API列表仍留待再次确认。
- 未来验收：内网IP、localhost、局域网域名和公网HTTP/HTTPS均有代表性验证；合法TLS与证书错误分别测试；未授权页面仍拒绝。当前不修改源码或运行网络请求验证。

### R08 补充问答：Bridge 什么时候会断线

- 三种Bridge中只有WS是长期连接；同步XHR是逐次HTTP请求，IPC是平台消息/回复，不应统一称为一条长连接。
- 已建立WS可能因页面完整刷新/导航、iframe移除、窗口关闭、WebView或Native进程崩溃/退出，以及本地socket读写错误而关闭。普通DOM变动或仅替换模块的HMR不应直接等同于页面重载。
- URL不可用、服务尚未监听、CSP拒绝、错误token/Origin、hello不匹配属于建连失败；与已经连上后断开分开报告。
- 本地Bridge连接127.0.0.1；外部网站不可达或互联网断开本身不是本地WS断开的充分条件。API超时也不等于连接断开。当前WS pump的10ms read timeout/WouldBlock会继续循环，不是10ms空闲断线。
- 当前JS close事件拒绝pending、清队列并约1秒后尝试重连；Rust退出该连接pump时cancel_connection。重连是新身份，不自动恢复旧文件handle/socket/非detached任务，也不重放结果未知的副作用调用。具体资源清理可靠性仍受已有批注影响，不将代码路径当成完整验收。
- XHR表现为请求失败/服务拒绝/文档消失；IPC表现为handler不可用、frame/窗口销毁、回复失败或超时。它们的失败契约在API总表复核时逐项明确。

本轮只更新review档案。主线程依据已审查的连接代码做针对性复核，没有修改代码、启动服务或实施上述设计。

### D12：Bridge 会话断开即双端清理、调用失败

日期：2026-09-25。用户明确决定：Bridge断线，两边都清理当前资源，API该报错就报错。状态：已决策待实现；关联D09/D10、AR-005、AR-009至AR-014。

统一契约：**确认会话断开 → 旧会话立即失效 → 两端各自清理所属资源 → 未完成调用失败；重连建立新会话，不恢复旧资源，不重放旧请求。**

- JS端：将会话资源对象标为失效，拒绝全部pending Promise、经错误回调/流错误结束对应操作，清空发送缓存和相关监听/计时器。保留旧对象引用也不能继续调用；断线期间需要该Bridge的新调用明确失败，不静默排队等重连。
- Native端：撤销该Owner/session的句柄，取消在途任务，关闭文件/socket/listener/watcher/管道并终止回收所属子进程。清理不依赖JS先发close；JS页面已经消失时，Native仍独立执行这条路径。
- 清理以会话归属为边界，不因一个iframe连接断开误清其他仍存活窗口/frame会话。应用/窗口级共享对象的归属应在统一资源模型中明确，不能通过随意扩大清理范围实现“全部”。
- 不需要成功完成一次“双端协商”才能清理；两端各自在观察到会话失效时执行。页面/进程已经终止的一端无法再跑回调，存活端不能等待其确认。
- 先撤销使用权，再完成实际释放；有耗时的进程终止/回收时不承诺瞬时完成。清理幂等；某项释放失败仍继续处理其余资源，并记录失败，不能因第一个错误留下其余资源。
- 旧会话的迟到响应、事件、finalizer或主线程任务不能作用到新会话；资源需有不可混淆的会话身份/代际。尚未开始的旧任务不得在失效后新增资源；已执行副作用不宣称自动回滚。
- 重连后调用方须重新创建资源、显式发起新操作；不能把旧调用改走IPC继续执行或自动重试有副作用请求。IPC是否支持某API仍按待复核清单，和本条失败规则分开。
- 本条不以detached标记自动豁免仍属于该会话的资源。若后续确需会话外独立任务，必须明确其所有权移交和清理契约；此前D10关于detached例外的笼统描述不能覆盖本条严格规则。
- 纯JS计算与既有静态快照不是需要Native关闭的资源，仍按各自API契约处理。
- 三种Bridge的应用方式：WS close可触发会话终止；同步XHR/IPC按页面上下文/原生会话有效性管理。单个API业务错误、HTTP目标失败或一次调用超时，不自动等同于整个Bridge会话断开；确认通道/会话不可用时才触发本条全量清理。

后续验收：主动断线、页面刷新/导航、iframe移除、窗口关闭、服务退出；双端资源计数回到该Owner清理后状态；每个pending只结束一次；旧对象立即报错；重复清理无误伤；重连无旧任务恢复/重放；异步清理和迟到事件不污染新会话。当前已有WS close/cancel_connection路径仅是实现基础，不能据此宣称上述完整契约已满足。

本轮只追加review决定，没有修改代码或执行测试。

### R09 / 第 1 轮：统一页面运行时的装载与模块入口

日期：2026-09-25。用户再次明确只记录review不动代码后要求继续。本轮只读源码并更新本档，不构建、不迁移、不改配置。主线程核查Rust内嵌/注入，Luna核查模块注册与ESM入口。

**现有结构与目标**：当前initialize_script.js负责Niva对象、Bridge、事件、模块注册表；packages/node-compat提供Node风格实现与ESM facade。D03要求未来合并为TypeScript源码的@niva/runtime，并产出注入JS及@niva/types；当前目录/源码尚未迁移，不能描述为已经完成。

**构建阶段**：crates/niva/build.rs读取runtime-files.json，按classic顺序拼接脚本，再加registerNodeCompat启动代码；同时收集ESM入口与依赖资源，生成压缩数据和索引供Rust include。该运行时资产按条目压缩/索引，与R03所述业务资源整包压缩的格式不同，不能混用读写器。

**页面阶段**：Wry初始化脚本先建立Niva及传输基础；符合条件的页面装入compat脚本，创建/注册模块，importmap将fs/node:fs等裸导入映射到本地ESM资源。业务脚本随后取得模块并调用。明确的debug入口另有初始化脚本装入路径，不代表远端HTML已被代理或改写；不同入口的就绪顺序留实际验收核对。

| 获取入口 | 现有机制 | 设计边界 |
|---|---|---|
| Niva.require / 注入的require | 同步读取已注册模块对象，工厂首次访问创建并缓存 | 不是从磁盘按Node规则任意解析node_modules，不提供完整Node CJS宿主 |
| ESM import | 浏览器依据importmap加载对应ESM facade | facade应取同一页面runtime对象，不再创建平行实现 |
| Niva.import | 已注册时Promise包装注册表读取；否则按资源映射动态import | 返回Promise是装载形式，不代表内部每个方法是异步Native API |
| 目标Niva.fs等 | D01/D03统一命名空间，默认提供相关API对象 | 与兼容入口同实现，不依赖是否注入Node全局环境 |

**实例与别名规则**：同一页面JS上下文中，应满足Niva.fs.readFile与require('fs').readFile、相应ESM函数来自同一实现；node:前缀和fs/promises等子入口按同一套对象关联。跨窗口/页面是不同JS上下文，不能要求对象引用跨上下文相等。对象被require到、类型检查通过，不表示当前Bridge/权限允许其所有Native操作。

**模式划分继续按D01**：基础模式只提供统一Niva命名空间；兼容模式额外提供require/module/process等Node环境及importmap。process/Buffer/require分别何时注入、已有同名全局如何处理需一致，而不是某个脚本各自决定。本轮不重新决定兼容环境开关的缺省值；用户只明确基础API默认提供。

**模块选择**：当前选择影响注册名、importmap和允许读取的ESM资源，classic脚本仍由整份清单拼接。不能把模块选择当成完整物理裁剪，也不能当成原生权限沙箱。Node/npm依赖需在构建时处理；提供require名字不代表运行时能直接执行任意npm包。

**与Devtools的衔接**：Vite接入最终指向同一runtime；需要时将fs等导入映射到Niva对应对象。避免打包器额外塞入另一套fs/process shim导致看似同名却状态、错误或函数身份不同。是否需要映射取决于实际开发/生产构建验证，R12继续讨论。

源码：crates/niva/build.rs、app/node_compat.rs::imports/classic_script/rewrite_html、assets/initialize_script.js::nivaRequire/registerModuleFactory/import；packages/node-compat/runtime-files.json与src/runtime/registration.js。R09状态：讲解中，无本轮运行验证。

#### AR-016：用户importmap覆盖与统一API身份要求需要明确规则

- 归属：R09/R12；类别：设计取舍；状态：接受现状（用户明确承担覆盖后的差异）；优先级：P2。
- 证据：node_compat.rs::rewrite_html使用imports.entry(name).or_insert，已有用户映射优先；initialize_script.js::Niva.require/Niva.import优先读本地注册表。用户将fs映射到别的模块时，静态ESM import与require可能得到不同实现。
- 影响：与D01要求各入口同一API对象/函数存在冲突条件，不能仅因为有ESM facade就声称无条件保证身份一致。
- 用户决定（2026-09-25）：用户自行覆盖是用户的事情，保留用户覆盖，不新增保留名禁用规则。统一API身份保证限定于框架默认提供且未被用户替换的入口；不替用户强行纠正映射。
- 未来验收：无冲突导入身份一致，冲突映射行为明确且可诊断，开发与打包环境规则一致。

R09 Luna 补充：默认root.Niva路径下，注册器复用runtime.fs/path/process等对象，ESM facade默认导出取同一对象；个别具名导出如process.on做了bind，不能未经逐项核对就把所有具名导出都宣称为相同函数引用。传入另一个非全局Niva对象注册时会新建适配模块，ESM facade仍读取全局runtime，这个多实例入口的保留范围留D03整合核对。

当前全局注入并未完全实现D01的统一开关：initialize_script总提供Niva.require，并在window.require不存在时安装全局require；注册buffer时仅在已有Buffer缺失时设置全局，注册process则设置root.process。模块选择主要筛选注册入口，不是独立的无全局污染模式。具体差距归入AR-002/D01/D03，不在review阶段修正代码。

注册器将同一模块对象注册普通名与node:别名，promises/strict子入口随父模块；zlib延迟factory首次访问初始化。Rust配置层拒绝未知模块名，但直接JS注册器对数组中的未知名以筛选方式忽略；后续统一配置/错误契约时需核对这两种入口，不把它们描述成已有一致校验。

本轮luna_deep_worker（/root/api_js_inventory）交付注册表/ESM/全局注入事实，主线程整理构建装载链、D01/D03目标差距与AR-016。只读核查和review档案写入，无代码/配置改动、无构建或测试。

### D13：Node 环境集中注入，模块只负责 Niva API

日期：2026-09-25。用户明确决定，状态：已决策待实现，仅review记录。

- Node兼容环境由统一启动/注入入口负责，集中处理require、process、Buffer、兼容模块环境和importmap，不允许各API模块分散写入这些页面全局变量。
- 其他模块只负责实现并提供Niva对象上的API及其内部依赖，不承担Node环境是否启用的决策。兼容入口从Niva的同一实现建立映射，默认未覆盖时满足D01的一致性。
- 用户自定义importmap/模块覆盖保持用户选择，AR-016已接受该行为；映射后的兼容性由用户负责，框架不新增禁止覆盖规则。
- CommonJS的module/exports/require/__filename/__dirname应由统一loader为每个模块创建各自的作用域，不能只给整个页面放一个共享module.exports对象。集中维护注入逻辑不等于所有模块共享同一局部状态。
- 本轮没有实施注入集中化，也未修改模块文件。

### R09 require 的 Node.js 兼容范围与成本评估

日期：2026-09-25。用户提出把require做成Node.js兼容并询问成本。方向记录为需求；精确版本、支持边界及验收范围尚待确定，本轮不是实施授权。

定性工程评估（不是工期或体积测量）：

| 范围 | 主要工作 | 成本判断 |
|---|---|---|
| 内置模块/已注册模块 require | 普通名/node:别名、缓存与同一Niva对象引用 | 较低，现有注册表可复用，但不等于完整CommonJS |
| 常用纯JS CommonJS包 | 每模块作用域；相对路径；.js/.cjs/.json；node_modules向上解析；package.json main/type/exports/imports等所选规则；缓存、循环依赖、require.resolve及错误处理 | 中等偏高；可复用现有fs/path/同步通道，核心新增成本在loader语义与验证 |
| 完整对齐某版Node加载行为 | 更完整条件导出/自引用/符号链接、缓存边界、ESM互操作及Node模块运行环境 | 高；WebView引擎限制和Native addon ABI不是增加一个JS require函数能解决的 |

- require必须保持同步。运行时从本地目录读取CommonJS源码，需要可用的同步资源读取/Native路径；只有异步IPC的页面不能用Promise冒充require。已经预注册/预打包的模块可同步读取，但不能将此能力冒充任意文件加载。
- CSP是实质实现约束：用new Function/eval包装运行时取得的源码会受script-src约束。需要明确采用可信本地模块的运行时执行机制或构建时预编译模块工厂；不能为了声称兼容而默认放宽所有页面CSP，也不能在有Native权限的页面执行网络获取的不可信代码。
- 浏览器原生ESM加载是另一套机制；require(ESM)的同步互操作需单独定义，不承诺直接复用dynamic import就符合require同步返回。
- `.node`原生扩展需要Node相关运行环境/ABI支持，不属于纯JS CommonJS loader能够兑现的兼容范围；所加载npm包依赖的Native API也必须在Niva支持范围内。
- 现有npm dependencies与解析器不等于已有完整loader。实现前应核对现有依赖/可复用维护库，避免手写全部解析细节；本轮未选型、未安装依赖。
- 建议将可验收目标明确为“选定Node版本的纯JS CommonJS加载规则”，覆盖本地文件、包解析、缓存/循环与模块作用域；ESM互操作、原生扩展等显式列出边界。这是一项长期模块能力，不是临时字符串替换；不估算未经验证的人天。

资料：Node官方CommonJS文档 https://nodejs.org/api/modules.html （本轮页面为v26.10.0，作为规则复杂度参考，不自动选为Niva验收版本）；MDN script-src对动态代码执行的限制 https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Content-Security-Policy/script-src 。建议验收用固定版本的模块加载用例与代表性纯JS包，分别报告加载规则、依赖API和WebView执行行为，不能以require能返回对象宣称npm全兼容。

主线程完成针对性的官方规则核对和成本分层；没有代码、配置、依赖变更，没有运行测试或构建。

### D14：以完整 CommonJS require 规则为目标，评估 Rust 加载后端

日期：2026-09-25。用户希望既然提供Node兼容环境就尽量完整兼容，提出把相关部分放到Rust，并明确不接管import。状态：兼容目标与ESM职责边界已记录；Rust/JS具体分工为评审建议，待后续方案与验收确认。只记录，不实现。

- 相比上一轮“常用子集”的建议，本轮按用户要求提升为对齐选定Node版本的CommonJS require规则；不能以简单内置模块注册表宣称完整兼容。具体基准版本仍需固定。
- 不自建/接管浏览器ESM import执行器，交给WebView及项目构建工具。D13已有的集中环境注入与映射配置不等于重写ESM加载器；本轮也没有自动删除已有importmap能力。
- 建议Rust负责路径规范化、相对/绝对定位、node_modules层级查找、package.json及条件入口解析、源文件/JSON读取、文件系统及包资源后端适配；优先评估可复用解析库，不预设全部手写。
- JS/WebView负责每模块函数作用域、模块执行、module.exports对象、缓存对象和循环依赖中的部分exports。真实JS函数/对象应留在同一页面上下文，以保持与Niva API的引用一致性；Rust可以缓存解析和源码结果，但不能用JSON替代活的JS模块实例。
- require同步获取模块代码后，仍需在页面中执行它。用syncXHR向Rust查询，再要求Rust同步回调同一被阻塞页面来执行JS，可能形成等待环；Wry evaluate_script的存在不能证明这条同步执行链可行。建议返回源/模块描述给页面后由页面执行，执行方式仍需解决CSP约束。
- 将解析和读取搬到Rust不自动解除new Function/eval的CSP限制，也不自动获得Node的V8/Node-API运行环境；不能因此承诺所有npm包、原生.node扩展或依赖Node内部机制的库可运行。
- 用户后续已明确排除`.node`原生扩展，其不再是待确认或待补齐项；CommonJS兼容目标限定在JavaScript/JSON模块。ESM import不接管；require遇到ESM的具体报错/边界单独记录，不据此承诺同步执行ESM。
- 只有IPC的页面仍不能以异步结果冒充同步require；内置/预注册模块可用与动态读盘模块可用须分开。Native加载后端继续执行来源及资源授权，不接入外部不可信脚本。
- 若另嵌JS引擎或Node执行模块，会引入与WebView不同的JS上下文、对象跨界和分发体积问题，是另一项架构选择；本轮不采纳也不实现该方案。

成本更新：完整CommonJS解析与加载规则是独立模块，成本高于当前注册表；Rust利于统一资源读取和复用解析库，但不消除模块语义、CSP、跨平台、生命周期及一致性测试成本。此处为基于职责拆分的定性评估，无实测工期/体积数据。目标应通过固定Node版本的加载用例验证，运行环境/API兼容另行验收，不以加载器通过替代完整Node兼容结论。

来源：Node官方CommonJS模块文档 https://nodejs.org/api/modules.html#file-modules ；Wry WebView执行脚本API https://docs.rs/wry/0.57.0/wry/struct.WebView.html#method.evaluate_script 。主线程核对接口后记录职责建议，没有改动代码、配置或依赖。

### D15：require/module 兼容层通过同步 XHR 加载

日期：2026-09-25。用户进一步明确D14的实现分工。状态：已决策待实现；本轮仅记录。

- 页面提供同步require及CommonJS module兼容层。需要加载未缓存的文件模块时，通过现有同步XHR Bridge请求Rust解析模块路径并读取内容；Rust返回规范模块标识、类型、源内容等必要信息。
- XHR返回后，由页面侧兼容层执行CommonJS模块并同步返回module.exports。Rust不在该XHR请求里反向等待WebView执行JS，避免同步等待环。
- 每个模块有独立module/exports和绑定自身位置的require，并提供__filename/__dirname；缓存和循环依赖所需的模块对象保留在页面上下文。执行前登记模块缓存，使循环引用可获得部分exports；失败时按所选Node基准处理缓存和错误。
- 内置模块直接复用Niva对应API对象；已缓存模块直接返回缓存，不要求每次require都发XHR。JSON模块按模块类型解析。
- Rust承担模块解析与资源读取，具体Node解析规则按D14目标核对；集中环境入口按D13安装兼容层，普通API模块不分散注入require/module。
- 不接管ESM import；`.node`原生扩展已明确不支持，遇到时明确报不支持错误，不尝试加载或回退到其他宿主。require(ESM)不在此处承诺兼容，其错误契约在后续收口时明确。
- CSP对页面执行动态源码的限制仍需在实现方案中验证；同步XHR解决获取源码，不自动保证执行方式可用。此为验证项，不改变用户已确定的XHR分工。
- 通道不可用或会话失效时，未缓存且需要Native加载的require同步抛错，不能返回Promise、偷偷改走异步IPC或挂起等重连；缓存模块内的资源失效规则沿用D12。
- 后续验收：内置/缓存模块身份一致，本地及包内文件解析、每模块作用域、循环依赖、JSON、错误和断线行为符合明确契约；不把require加载成功等同于包所用全部Node API已兼容。

本轮只追加review档案，没有修改源码、配置、依赖或运行测试。

### D14/D15 范围收口：JavaScript CommonJS，不支持原生扩展

日期：2026-09-25。用户明确“.node之类的肯定不管”，要求整理记录。本条与更新后的D14/D15共同作为后续实现依据，替代原先将.node列为待确认的措辞。

| 项目 | 决定 |
|---|---|
| require/module兼容层 | 实现JavaScript CommonJS加载与执行契约，支持JSON模块 |
| 内置模块 | 从Niva统一API对象获取，默认未被用户覆盖时共享实现 |
| 文件模块加载 | require同步XHR请求Rust解析/读取，页面执行后同步返回module.exports |
| 模块语义 | 独立module/exports、局部require、__filename/__dirname、缓存与循环依赖 |
| 包解析 | 以选定Node基准的CommonJS规则为目标；具体版本及用例后续固定 |
| Node环境注入 | 集中在统一入口；其他模块只负责Niva API |
| 用户映射覆盖 | 尊重用户配置，由用户承担覆盖后的行为差异 |
| .node及Node原生扩展加载 | 明确不支持，不是后续待补齐能力；遇到时明确报错 |
| ESM import | 提供Node风格API的.mjs入口及import map，模块加载/执行遵循浏览器原生ESM标准；详见D16 |
| require(ESM) | 本次不承诺兼容，后续只需明确边界/错误行为，不自动扩展为自建ESM执行器 |

不为原生扩展自动引入Node宿主或另一个JS运行环境。此处的“.node扩展”与现有Rust RPC方法名fs.node无关，后者仍是文件操作分发入口。兼容层支持包加载不代表所有包使用的Node API已经实现，API范围仍依总表审查。

本轮只整理review档案，未修改代码、配置、依赖，也未运行构建或测试。

### D16：ESM 使用 Web 标准，提供 .mjs API 入口与 import map

日期：2026-09-25。用户明确：import只提供Node模块映射，import标准使用Web自身标准，提供一套.mjs API import map即可。状态：已决策待实现，仅review记录。

- 框架提供Node风格API的.mjs facade和对应import map，例如fs/node:fs及支持的fs/promises等子入口映射到相应.mjs资源；以实际支持模块清单生成，不宣称Node所有模块均有实现。
- facade从同一页面的Niva统一API对象导出默认对象和具名API，不复制实现、不再建立一套Native连接或资源对象。用户未覆盖时，Niva.fs.readFile、require('fs').readFile及ESM相应导出应遵守同一实现/身份约定。
- 静态import与动态import()的解析、加载、模块执行和缓存由WebView按浏览器ESM标准处理。不在Rust重建ESM引擎，也不把ESM import改造成同步XHR require。
- 模块资源由现有资源服务提供，使用正确JavaScript MIME类型；运行时和映射需按浏览器模块装载顺序就绪。具体资源URL和产物布局留D03实现方案确认，本轮不提前固定目录。
- Node兼容环境及import map的注入仍由D13统一入口控制；API模块只负责Niva API。.mjs facade是导出入口，不承担全局环境注入。
- 用户自己的映射覆盖保持优先，由用户承担覆盖后与内置入口的差异；不强行覆盖回框架版本。
- 此映射不提供浏览器原本没有的任意npm包自动解析，也不承诺require(ESM)或.node扩展；CommonJS路径继续按D15使用同步XHR加载JS/JSON模块。
- 当前源码的ESM facade后缀为.js、映射也指向.js；本条.mjs布局是后续目标，不声称已经迁移。

验收方向：使用浏览器原生import/import()取得支持的模块，默认/具名及子路径导出正确；未覆盖时与Niva/require共享实现；用户覆盖受尊重；资源MIME、初始化顺序、开发/打包入口行为一致。具体API支持性和IPC fallback仍按总表另行确认。

本轮只更新review档案，没有修改代码、配置、依赖或运行构建/测试。

### D17：Niva 为唯一 API 实现来源，CommonJS/ESM 仅导出接口

日期：2026-09-25。用户明确最终关系：所有API实现在Niva上；自定义require直接注册自己的Node CommonJS接口；CommonJS和ESM只是从Niva对象导出的interface。状态：已决策待实现，仅review记录。本条细化D01/D03/D13/D15/D16，不另建平行实现。

| 层 | 唯一职责 |
|---|---|
| Niva API对象 | 承载全部公开API实现及其状态、资源与Bridge调用；可在内部按模块组织源码 |
| CommonJS内置模块接口 | 将Niva对应模块对象/函数注册到自定义require，例如fs/node:fs；只暴露同一实现，不复制逻辑 |
| ESM .mjs接口 | 从Niva对应模块导出默认对象及具名API，配合import map由浏览器加载；只导出同一实现 |
| Node环境注入入口 | 统一安装require/process/Buffer等兼容环境与映射，不由各API文件分散注入 |
| 用户CommonJS文件加载器 | 仅对未缓存的文件模块走D15同步XHR解析/读取；内置接口直接返回已注册引用，不额外请求Native |

- 这里interface是运行时接口/导出层，不仅指TypeScript类型声明。类型声明由D03同源生成/导出，不能代替运行时对象。
- 在同一页面且未被用户覆盖时，require('fs')直接取得Niva.fs对应对象，ESM默认fs导出也取该对象；对应readFile等函数引用一致。Node子路径（如fs/promises）同样导出Niva对应子对象。
- API语义、错误转换、资源生命周期和传输选择只在Niva实现层维护。CommonJS/ESM接口不另建缓存状态、Native资源或Bridge客户端；模块加载器自身的缓存与模块作用域仍按CommonJS/ESM各自机制管理。
- 函数需要绑定所属对象时，在统一Niva实现层统一处理；不由CJS/ESM分别bind生成不同函数而无意破坏身份约定。用户覆盖行为继续按AR-016允许，不要求框架恢复其改动。
- 不为内置模块加载再执行一份独立Node实现；也不因源码分成多个文件，就把“所有API在Niva上”误解成所有源码必须塞在一个文件。
- 当前分散实现/注册与新目标的差异继续按API表和D03统一runtime整合处理；本条不是现有源码全部满足该结构的验收结论。

本轮只追加review档案，没有修改源码、配置、依赖或运行测试。

### D18：CommonJS 与 ESM 环境采用两个独立配置开关

日期：2026-09-25。用户明确：注入require时直接注册Node内置模块interface；配置拆成“注入Node.js CommonJS环境”和“注入ESM环境”两个可选项。状态：已决策待实现；覆盖D01/D13中把兼容环境视作单一注入开关的笼统描述。仅更新review档案。

- 基础Niva API始终按已定范围提供，两个开关均不控制API是否实现、不复制实现，也不改变原生权限。
- CommonJS开关：启用时由统一入口安装自定义require及相应Node CommonJS环境，直接将Niva对应对象注册为内置模块interface（含node:及已支持子路径别名）。用户文件模块的同步XHR加载继续遵循D15；内置模块不走文件加载。
- ESM开关：启用时由统一入口注入Node API的import map，指向导出Niva接口的.mjs facade。ESM解析、执行与缓存仍由浏览器负责；该开关不负责安装require，也不改变浏览器自身支持import/import()的能力。
- process/Buffer等CommonJS兼容环境全局的安装归统一环境入口；ESM模块可以通过对应facade取得同一Niva对象，不应仅因为启用ESM映射就隐式开启CommonJS全局注入。

| CommonJS注入 | ESM注入 | 框架提供的使用方式 |
|---|---|---|
| 关闭 | 关闭 | Niva命名空间；不注入框架的require或Node模块import map |
| 开启 | 关闭 | Niva + require/module兼容层及内置模块注册 |
| 关闭 | 开启 | Niva + .mjs接口import map；使用Web标准ESM |
| 开启 | 开启 | 两套接口入口共用同一Niva API实现 |

配置字段的最终名称与两个开关各自的缺省值尚未由用户指定，不在本轮代定。所谓“关闭”只表示框架不注入，不删除用户或其他工具已经提供的require/import map。用户映射覆盖继续按AR-016处理。

验收：四种组合分别验证入口存在性、Node全局污染边界、默认/具名/子路径导出与函数身份一致性；开关不改变资源所有权、会话断线规则和API授权。当前配置尚未改动，不声称四种组合已经实现。

### D19：以真实复杂 Node 项目进行端到端兼容验收

日期：2026-09-25。用户提出用真实复杂Node项目验证兼容能力。状态：确定增加此验收层，具体项目和固定版本待选择；本轮只记录，不下载、安装、修改或运行项目。

**目标**：证明一个真实项目在已定义范围内能完整工作，而不仅是模块可以require、API名字存在或单元测试通过。结果表述限定到具体项目版本、功能路径、平台及运行环境，不能从单个项目推导全部Node/npm兼容。

**项目选择**：采用真实维护的应用/工具或用户现有项目，包含实际的多层依赖、配置、文件I/O、网络和错误路径；优先选择核心入口及所验收路径适用CommonJS、无需.node原生扩展的项目。先公开其依赖、Node内置模块需求与所需功能，再固定候选和版本；不能先挑跑通的少量功能再称作整个复杂项目兼容。具体项目尚未选定，不虚构候选完成度。

**固定基线**：记录项目仓库、commit/tag、lockfile、参考Node版本、Niva代码/构建身份、目标平台与参数。验收前列出必测业务流程、允许的最小宿主接入、明确排除项及通过标准。既定不支持项如.node单列，不静默删除失败用例或在运行后随意扩大排除范围。

**真实执行约束**：

- 被测代码在Niva页面运行时执行，由自定义require/module层解析加载CommonJS文件和依赖，使用Niva唯一API实现。可设置一个最小项目启动/结果采集入口，但不改项目业务源码来绕过失败。
- 原始CommonJS装载路径必须单独验收，不能仅用预打包后的浏览器产物替代，掩盖node_modules解析、模块缓存或循环依赖问题。若项目正常需要构建，记录其官方构建流程，并区分运行时加载与构建产物兼容证据。
- 宿主Node可以运行参考基线、构建工具或外部测试驱动，不得作为Niva被测模块/文件/网络/进程能力的隐藏实现。缺失模块不得回退到宿主Node require。
- 收集模块解析记录及相关Native调用证据，证明关键业务链实际走Niva及三种Bridge对应路径。诊断日志不得泄露token或秘密；保持项目源码与测试数据隔离。

**流程覆盖建议**：启动与依赖加载、配置读取/写回、目录/文件操作、真实HTTP客户端/服务端交互、内网及外网目标、代表性子进程操作（项目实际使用时）、错误/权限/超时、重复操作，以及Bridge断线后的资源清理。目标功能由项目真实需求决定，不为凑覆盖伪造业务。

**对照与证据**：同一项目版本在参考Node与Niva执行相同输入，对比业务输出、协议结果、文件副作用与错误语义；对时间/随机性等非确定输出提前定义比较方法。报告成功、失败、未覆盖及明确不支持项，附复现步骤、输出和资源清理结果。

**平台与模式**：macOS和Windows分别实测，不能用target check代替。先验证完整本地能力路径；CommonJS/ESM环境开关按D18另验入口组合。IPC fallback在清单冻结后验证对应功能子集，不能因为完整项目依赖流/同步就宣称IPC应全量兼容，也不能只测IPC子集就声称全项目通过。

**在整体门禁中的位置**：真实项目端到端验收与固定版本的Node模块/加载器契约测试互补，前者证明业务路径，后者定位边界及回归；两者都保留。性能和完整产物体积另测，不从项目跑通推断达标。

下一步（未来实现/验收阶段）：选定具体项目和版本，先做依赖及功能清单，再制定可复现的Node/Niva对照运行方案。本轮未选定项目、未执行上述验收，没有任何代码变更。

#### D19 候选推荐 — 2026-09-25：http-server 主样本 + JavaScript 版 tsc 补样本

用户询问具体项目选择。本轮只读官方仓库/文档筛选，以下为推荐，尚未固定测试版本、安装或运行；不把候选推荐当成已经兼容。

**主样本建议：http-party/http-server。** 当前官方源码使用CommonJS，实际依赖union/http-proxy等模块，核心会用fs/path/url/stream/Buffer以及HTTP/HTTPS。适合同时检查require包解析、多层依赖、同步/异步文件操作、流式响应和Native网络，不是仅启动后返回一句话的演示。

预先列定的代表性流程候选：

1. 通过Niva的CommonJS loader启动原项目，提供固定静态目录。
2. 请求HTML、CSS、图片、二进制文件与目录列表，核对内容和状态码。
3. 验证HEAD、Range、条件缓存/304、404等HTTP语义。
4. 验证HTTP代理、HTTPS证书及基本认证等明确选定功能。
5. 并发下载/中途断开/Bridge会话终止，核对文件流、listener、socket清理与重新启动。

完整依赖树仍需锁定版本后核对；不能只因顶层package.json未出现.node就保证所有依赖符合范围。当前读到的是仓库master源码，package.json标14.1.2，不将此等同于已选定的正式发布版本。基本可行性未实测。

**复杂补样本建议：JavaScript版TypeScript编译器tsc。** 使用原编译器在Niva中编译一个真实多文件/多项目引用的TS工程，对照Node的输出文件和诊断，再补增量/监听流程。它适合检验大量同步文件访问、路径和编译计算，但编译器发布产物的打包结构不能单独证明完整node_modules动态加载，因此与http-server互补。Node专有性能/诊断等可选接口和watch语义需按固定版本核查；不使用原生编译器替代被测JavaScript入口。

**本轮不推荐Node-RED作为主样本**：官方Function节点直接require('vm')并使用vm.Script，超出当前已排除的vm模块范围。不能删去此核心需求后泛称整个Node-RED通过，也不为挑选验收样本擅自扩大Node兼容目标。

官方依据（2026-09-25只读查询）：

- http-server manifest及入口：https://github.com/http-party/http-server/blob/master/package.json 、https://github.com/http-party/http-server/blob/master/lib/http-server.js
- 静态资源、Range、文件流实现：https://github.com/http-party/http-server/blob/master/lib/core/index.js
- TypeScript项目引用流程：https://www.typescriptlang.org/docs/handbook/project-references.html
- Node-RED Function节点的vm依赖：https://github.com/node-red/node-red/blob/master/packages/node_modules/@node-red/nodes/core/function/10-function.js

建议优先顺序：http-server真实应用链路，再加tsc复杂文件/计算链路；若只选一个，先http-server。来源选择和最终锁定版本仍需后续确认；本轮不执行验收、不修改代码。

#### D19 项目方案确认 — 2026-09-25

用户要求“记录一下方案，然后继续下一章”，据此将推荐方案记为已选验收项目组合：http-server作为真实多依赖网络/文件应用主样本，JavaScript版TypeScript tsc作为复杂编译与文件系统补样本。具体发布版本、依赖锁、被编译的真实TS工程和必测用例仍需后续固定。当前未安装、运行或改动这些项目。

### R10 / 第 1 轮：兼容语义、证据层次与验收分母

日期：2026-09-25。用户要求推进下一章；R09设计决定D13–D18保留。R10不运行测试，只审查验证结构及现有记录，主线程核对报告，Luna核对上游runner。

| 证据层 | 能证明什么 | 不能单独证明什么 |
|---|---|---|
| 入口/类型检查 | 模块或方法存在，参数静态类型能匹配 | 行为、错误、选项、资源释放与Node一致 |
| 自有单元测试 | 已写用例的实现逻辑满足预期 | 若使用Bridge doubles，不能证明真实Native链路 |
| 固定版本Node上游用例 | 明确选中的原测试对被测实现的行为要求 | 未选用例、所有模块契约、全部WebView引擎行为 |
| 真实Niva/WebView集成 | 真实页面、Bridge、Native与平台协作 | 所有API选项/错误/平台均已覆盖 |
| http-server + JS版tsc真实项目 | 固定项目和业务流程在Niva中可完成 | 所有npm包或所有Node能力均兼容 |
| 平台与分发验收 | 指定平台及实际产物的运行与分发行为 | 另一平台、另一产物或不同源码版本同样通过 |

**一个API的兼容不止名称**：需核对参数与选项、同步/Promise/callback返回方式、事件顺序、数据类型和编码、错误code/errno、取消/超时、重复调用、资源释放、跨平台差异。例如fs.readFile存在，不代表默认Buffer、utf8、失败回调、AbortSignal及大文件边界均符合；http.request返回对象，不代表Agent/连接池/upgrade等所有选项已支持。

**与当前设计决定对应的必测契约**：D01/D17默认未覆盖时API身份一致；D18四种环境配置组合；D15 require同步加载、缓存/循环与模块作用域；D16浏览器ESM facade；D12断线双端清理、旧资源失效、无隐式重放；D11内外网HTTP策略；IPC清单冻结后逐项验证其JSON/非流式边界。

**现有历史证据的准确读法**：

- docs/node-api-implementation-evidence.json记录22模块与179目标入口等历史观测，不覆盖此次新增require loader和统一注入目标；API总表437行/组含多个层级，也不是可以拿来与179直接计算增长率的同一分母。
- docs/node-compat-upstream-results.json记录2026-09-24、固定Node v22.14.0、58个文件pass，附2处精确环境豁免。豁免不当作对应断言已通过；58是文件计数，不是179项API全契约通过。
- docs/node-upstream-unfiltered-results.json记录同日58文件中56 pass/2 fail，保留未豁免诊断。两份报告的Native binary SHA不同，不能把结果差异全部归因于“仅切换豁免”，也不能宣称同一冻结产物的A/B实验。
- 当前docs/node-webview-engine-boundary-evidence.json明确是固定Node oracle的公开可观察性诊断，不是真实WebView验收。两项环境差异涉及删除原型后的构造器名，以及原生CryptoKey内部材料的同步比较；应在豁免说明中保留，不能通过针对测试写死结果或删除整个模块来美化通过率。
- 本轮没有复跑，上述只能称为仓库中的历史记录，不能把“存在通过报告”当成正在review的代码版本已验收。

**后续报告统一写法**：记录固定测试集/版本、源码与runner hash、实际执行载体、Native产物hash、平台、通过/失败/不支持/未覆盖、逐条排除或豁免及理由。参考Node可作为oracle/外部测试驱动，但缺失的被测API不能悄悄落回Node实现。每次scope变更更新分母，并保留未过滤结果。

**真实项目验收**：D19项目按原始CommonJS路径运行，尽量不改业务源码；启动成功只是第一步，必须对照文件内容、网络协议、诊断/构建产物、错误与资源清理。http-server主要验证多依赖/网络流，tsc补同步文件/计算，两者也不替代上游边界用例。macOS和Windows分别验证，不用target check冒充真机。

#### AR-017：历史验收文档的“当前”统计需要统一标注

- 归属：R10/R14；类别：文档不一致；状态：已确认待决策；优先级：P2。
- 证据：docs/node-compat-implementation.md开头写58文件及豁免结果，后面“明确限制与后续门禁”仍写“当前选定的30个官方文件全部通过”；同页当前/历史措辞易混淆。node-compat-test-matrix.md还保留较早模块/API范围，不能当本次源码全集。
- 影响：读者可能混用不同阶段的分母与执行范围；不因此否定原始JSON记录，也不推断未测能力通过。
- 后续：实现/文档收口阶段给历史段落明确版本与时间，当前摘要从同一冻结报告生成或逐项核对；本轮不修改这些原文档。

R10状态：讲解中，等待用户讨论。未执行测试、构建或真实项目运行。

R10 runner执行载体核查（Luna只读证据）：

- test-upstream.mjs在固定Node v22.14.0子进程中通过vm.runInThisContext包装、执行原始CommonJS用例；测试JS并非全部在WebView引擎运行。选定Native用例通过Python loopback relay连接指定Niva binary/WebView，缺少NIVA_UPSTREAM_BINARY记unsupported。
- requireNiva把被测模块映射到Niva facade，未知模块拒绝，不存在通用createRequire兜底；断言/诊断/固定fixture的宿主测试设施例外仍需在报告中显式列出。另有检查防止path/Buffer/crypto误指向宿主实现。
- 固定58文件分为48个JS-host与10个Native，旧30文件基线保留；Node版本、upstream commit及文件hash固定。不能将58文件结果统称为“58个真实WebView JS用例通过”。
- 除两处明确环境豁免外，runner还存在按平台标记platform-excluded的node:test子测试；豁免语句、平台跳过子测试与文件级pass是不同计数，后续统一报告必须同时说明，不将“2处豁免”解释为所有层级只跳过两项。
- 这说明R10的WebView集成和D19真实项目验收必不可少：Native后端真实并不能把宿主Node的JS执行自动变为WebView执行证据。

来源：packages/node-compat/scripts/test-upstream.mjs约199/215行及docs/node-upstream-conformance.md。luna_deep_worker（/root/api_js_inventory）完成runner静态核查；主线程交叉整理历史JSON报告、分母与章节说明。未复跑结果，也未修改代码。

#### D19 修订讨论 — http-server 不再作为主验收基线

日期：2026-09-25。用户认为http-server价值不大。撤回“已选主验收项目”的定位，最多保留为网络专项候选；尚不自动执行替代方案。原候选及资料保留为历史。

主线程建议把已有JavaScript版tsc提升为主验收候选：在Niva进程的页面运行时直接装载原版tsc，对真实Niva Devtools TypeScript工程进行类型检查，并以具有产物的真实TS工程补编译输出/增量验证。不能通过启动宿主node进程执行tsc来冒充Niva兼容；不把现代Vite自身的ESM执行与原版tsc测试混为同一任务。tsc对文件和复杂计算有实际用途，但其发布bundle不能独自验证完整CommonJS依赖解析，这部分继续由加载器契约测试补充。

如果用户希望另选带状态的应用级样本，新的备选是json-server v0.17.4（明确为旧版CommonJS候选，不是声称最新版本）：其官方版本说明和manifest提供Express/lowdb/body-parser/compression等依赖，CRUD请求会持久化到db.json，适合验证创建/修改/查询、过滤分页、落盘与重启重载。用原项目运行时测试，而不是重写一个简化CRUD服务。完整依赖锁及Native扩展需求仍需核查，当前未安装、运行或证明可用。

备选资料：https://github.com/typicode/json-server/tree/v0.17.4 、https://github.com/typicode/json-server/blob/v0.17.4/package.json 、https://github.com/typicode/json-server/blob/v0.17.4/README.md 。本轮仅记录用户否定及替代建议，最终替代项目待用户确认；没有代码或配置改动。

#### D19 Express 完整应用优先候选 — Cypress Real World App

日期：2026-09-25。用户要求找Express或基于Express的相对完整项目。主线程只读对照官方源码后，优先推荐 https://github.com/cypress-io/cypress-realworld-app 。这是候选推荐，尚未锁定commit或执行；不把此项目描述为已经在Niva可用。

**项目性质与适配理由**：官方说明为教学/测试用途的完整支付演示应用，不是真实生产支付系统。采用Express后端、React前端、TypeScript、lowdb本地JSON数据库，提供本地认证和现成E2E/API测试；后端包含用户、联系人、银行账户、交易、点赞/评论、通知和GraphQL等路由。业务数据层使用lowdb FileSync、fs/path、bcryptjs和多层依赖。相比静态文件服务器，更能验证带状态的应用工作流。

**CommonJS依据**：tsconfig.tsnode.json明确module=commonjs；后端源码是TypeScript，推荐在构建阶段按该语义编译为保留require依赖的CommonJS，再由Niva自定义loader执行，不能将ts-node/宿主Node运行后端当作通过证据。构建与Cypress外部测试工具可独立运行，其宿主与被测Niva后端必须区分。不是要求Niva直接执行TypeScript或运行Cypress自身。

**候选验收流程**：

1. 载入原前端产物，后端CommonJS及依赖在Niva内部执行，建立真实HTTP/会话链路。
2. 注册/登录/退出及会话保持，创建和修改账户/联系人。
3. 发起项目支持的模拟交易或付款请求，核对交易列表、过滤分页、相关评论/通知与数据库内容。
4. 验证JSON数据确实写盘并在不重新seed的重启路径重载；项目官方dev/start脚本会重新seed数据库，必须事前说明启动方式，不能把官方重置行为误报为Niva持久化丢失。
5. 覆盖无权限/非法参数/重复操作等错误，以及Bridge断线后的资源失效和后端重新启动。
6. 对照Node参考运行与原项目API测试；真实前端流程另行UI验收，不能只看HTTP根路由200。

**后续必须核查的接入边界**：完整锁定依赖是否含必需的.node/其他已排除能力；实际运行依赖并不等于package.json中dependencies一栏（Express等在该仓库devDependencies里），不能简单omit-dev后声称项目自身缺依赖；构建产物/路径布局、资源读取、启动全局、local authentication、前后端端口与CORS/会话Cookie要按原应用配置接入。第三方Auth0/Okta/Cognito/Google模式为另外的选项，不把未测试的第三方集成算作通过。上述为验收前核查项，不为跑通删改原业务源码。

当前只读证据（读取develop分支，未来需冻结commit）：

- README：完整Express/React演示、lowdb/local auth、默认seed行为：https://github.com/cypress-io/cypress-realworld-app/blob/develop/README.md
- 后端模块/路由：https://github.com/cypress-io/cypress-realworld-app/blob/develop/backend/app.ts
- JSON持久化与bcryptjs：https://github.com/cypress-io/cypress-realworld-app/blob/develop/backend/database.ts
- CommonJS编译配置：https://github.com/cypress-io/cypress-realworld-app/blob/develop/tsconfig.tsnode.json
- 依赖与测试脚本：https://github.com/cypress-io/cypress-realworld-app/blob/develop/package.json

对照候选：hagopj13/node-express-boilerplate偏完整认证/用户管理后端，需外部MongoDB；RealWorld Conduit旧CommonJS+Mongoose提交具备文章/关注等业务，但依赖年代较早，仓库当前主线已改为另一套Prisma栈，不能直接沿用旧版适配判断。此轮优先Cypress RWA，原http-server继续仅为专项候选，tsc保留补测建议；最终选择待用户确认。

本轮只读调研与更新review档案；未克隆、安装、构建、运行候选或修改任何代码。

Express候选Luna补充：luna_deep_worker（/root/architecture_inventory）核查hackathon-starter 10.0.0/b8f47ec：CommonJS入口，但直接依赖@node-rs/bcrypt原生绑定并连接MongoDB，不适合作为当前排除.node前提下的首个完整验收项目。依据 https://github.com/sahat/hackathon-starter/blob/b8f47ec/package.json 及 https://github.com/sahat/hackathon-starter/blob/b8f47ec/models/User.js 。主线程完成Cypress RWA、RealWorld与Express boilerplate对照和最终候选建议；均未安装或运行。

#### D19 主验收项目确认 — Cypress Real World App

日期：2026-09-25。用户对Cypress Real World App候选表示“可以，继续”，确认其为当前Express完整应用主验收项目。后续冻结commit、锁定实际运行依赖和业务用例，先以原后端CommonJS产物在Niva执行、本地认证及lowdb数据流程为基础；完整范围仍按D19规则预先声明。tsc保留补测建议，http-server不是主验收。当前没有克隆、安装、编译或运行任何候选。

### R11 / 第 1 轮：外部宿主与 stdio UI 子进程

日期：2026-09-25。当前推进R11，R10证据边界保留。主线程核查Python/页面示例和stdio保留规则，Luna核查Rust协议实现；只读代码、更新本档，不运行示例。

**定位**：任意外部程序（Python、Rust、Node等）启动Niva作为UI子进程，双方用stdin/stdout交换JSON业务消息。外部宿主协议不是页面→Native的第四种API Bridge：前者跨进程连接宿主和Niva，后者是Niva内部页面与Rust之间的WS/XHR/IPC三通道。

**正常流程**：宿主创建Niva进程并接管管道，传入stdio模式和资源/配置来源；Niva启动主窗口；主窗口WS hello后stdout输出ready；页面安装host:message监听器并发送page:ready；宿主收到后发送业务消息；页面处理并通过host.send返回业务结果。当前示例仍使用--debug-resource，D04目标改为--resource/--config，review阶段不更改示例代码。

| 方向 | 帧/接口 | 意义 |
|---|---|---|
| Niva → 宿主 | {"t":"ready","v":1} | 本进程首次主窗口WS握手就绪，不是业务页面已准备完成 |
| 双向业务消息 | {"t":"msg","name":"...","data":...} | 一行一JSON；业务payload由双方约定 |
| 页面接收 | host:message事件 | 当前投递到主窗口的事件通路 |
| 页面发出 | 当前Niva.api.host.send(name,data) | Rust要求window ID为0且以--stdio启动 |

协议是UTF-8 NDJSON，消息以换行分隔；不是HTTP，不包含原生API的call/result配对机制。需要请求关联时，业务自行在data中约定request ID、成功/失败响应与等待期限。不要把宿主msg字段直接解释成获得任意Native调用权限。

**就绪与刷新**：ready只输出一次；page:ready是示例业务握手，不是Rust隐式识别的协议类型。页面刷新后宿主管道可以仍在，但旧页面上下文已经失效，新页面需重新进行业务就绪握手。D12的旧会话资源失效/禁止重放规则仍适用，不自动把之前的业务请求恢复到新页面。

**归属边界**：当前按主窗口ID0控制，不等于仅顶层frame具有调用能力；主窗口的send_ipc_event会向其连接集合广播。子窗口调用host.send被拒；主窗口同源frame的接收/调用范围需按窗口/来源模型理解，不把主窗口限定误述成严格主frame隔离。IPC fallback单次调用能力也不等于有宿主事件推送。

**管道与日志**：--stdio模式下stdout只用于协议帧，日志经stderr。process.stdin及process.stdout写入API在该模式拒绝占用协议通道，stderr仍可写；这是Node兼容运行的明确模式限制。普通运行模式与stdio模式不可混用同一验收预期。

**退出与失败**：stdin EOF、读取失败或stdout写入失败会请求应用退出；request_stdio_exit投递到主线程，清理主窗路径后设置Exit，即使清理返回错误仍设置退出。父进程结束通常影响管道，但检测依据是实际EOF/写失败，不等于无条件即时检测任意父进程生命周期。坏JSON等输入记录错误并丢弃，不应直接破坏后续消息解析。

当前协议限制包括单行64MiB及有界stdout队列，不能据此当作可靠的大文件流通道。host.send的返回需要分清排队/写入/对方收到/业务完成，后续契约不应将这些阶段合并成“发送成功”。

源码：examples/stdio_host.py、examples/stdio-host/index.html；api/host.rs::send；api/process.rs::write_stdio/read_stdin；app/mod.rs::request_stdio_exit；app/stdio.rs。R11状态：讲解中，未进行本轮运行验收。

R11 Luna补充：stdout队列当前容量为1，host.send使用try_send，队列满立即报错；成功仅说明消息被队列接受，不保证宿主已经收到或完成业务。writer独立线程写帧并flush；协议没有ACK。输入超长行排空后继续解析，EOF前无换行的末帧仍处理。当前容量/快速失败是否为最终背压契约留用户讨论，尚不认定为实现缺陷或擅自调整。

luna_deep_worker（/root/architecture_inventory）核查Rust帧格式/限制/失败触发；主线程串联真实示例和主线程退出路径。所有结果为静态核对，未运行示例或修改源码。

### D20：宿主通信统一使用主窗口 process stdio，移除 api.host

日期：2026-09-25。用户提出不再需要api.host，直接使用仅提供给主窗口的process stdio。评审建议采纳并记录为后续设计方向；本轮仅review，不修改任何实现。以下目标替代R11中旧专用NDJSON宿主协议的API设计，R11原文保留为当前实现证据。

- 主窗口使用统一Niva.process.stdin/stdout/stderr；CommonJS require('process')、Node兼容全局process及ESM接口按D17复用同一对象。process作用域限制继续生效，不能因删除api.host放宽到其他窗口/frame。
- 移除公开api.host.send及专用host:message接口；不再并行维护两套宿主发送/接收API。关联旧类型、示例、文档和Native注册在未来实现阶段同步整理。
- Rust提供真实stdin读取、stdout/stderr写入与流生命周期；不再消费stdin并解析NDJSON后转成host:message，也不再只允许stdout输出ready/msg帧。现有ERR_STDIO_RESERVED检查与旧StdioBridge读写器必须一并重构，不能让两个reader争抢stdin。
- 消息格式由宿主与页面应用约定，可以继续使用NDJSON，也可以采用其他明确的流协议。Node流的data块不等于完整业务消息，应用需正确处理分片、合并、编码和边界。
- 若使用NDJSON，页面安装stdin监听后，通过stdout发送应用约定的ready消息；旧Rust自动ready帧不再作为隐式协议承诺。请求ID、响应、超时均由宿主业务协议定义。
- 按用户后续决定，Niva框架诊断走独立日志系统，不占用stdin/stdout/stderr；用户应用通过process使用标准流，不由框架假设stdout是某个固定JSON帧。此前“框架日志走stderr”的提议已被替代。
- process.stdout.write的返回/回调、drain/背压、stdin end/error、流关闭与Bridge D12清理应按统一Node风格契约验收；不能沿用旧host.send队列接收成功就宣称stdout已写入或对方已处理。
- 同一stdin只能有一个实际Native读取所有者，页面监听器在统一流对象上分发；页面刷新/Bridge断线需撤销旧读取者和资源，新会话是否重新接管管道按明确的重绑定规则处理。不能由旧读取者吞掉新页面消息。
- 用户后续确认：移除专用--stdio参数，不区分额外宿主模式。宿主按普通子进程方式连接stdin/stdout/stderr，主窗口通过统一process流使用它们；不以隐式检测管道恢复旧NDJSON模式。
- stdin EOF是否默认退出Niva、stdout BrokenPipe如何处理，以及宿主模式与普通Node程序stdin关闭后的差别，要单列确认。移除host API本身不自动决定这些进程生命周期政策。
- process stdio是流式/二进制能力，页面到Native继续依赖WS通道；不能因外层使用操作系统管道就推断IPC fallback需新增流能力。它仍只属于获准主窗口的process对象。

未来验收：同一原始stdio接口完成宿主往返，无host API依赖；NDJSON样例正确处理多块/多帧；框架日志不污染stdout；主窗口以外不可获得process权限；读者互斥、背压、EOF/BrokenPipe及Bridge断线无悬挂调用或重复读取。当前尚未实现或执行这些检查。

#### D20 补充：移除 --stdio 参数

日期：2026-09-25。用户进一步确认不再需要专门的--stdio参数。目标确定为：普通进程标准流 + 主窗口process API，删除旧模式开关、api.host及内置宿主帧协议；不保留旧参数兼容别名。

- 宿主使用标准子进程管道连接方式，无需知道Niva专用stdio启动参数。页面通过Niva.process或对应CommonJS/ESM入口使用同一组流。
- Niva框架日志走独立日志系统，默认不写stdout或stderr，也不依赖--stdio开关切换。用户应用如何使用标准流由其自行约定，详见D21。
- 没有控制台/管道可用时的读写及错误处理按平台核查，尤其Windows GUI程序继承标准句柄；不靠取消参数就宣称跨平台已验证。
- 不再沿用“stdin EOF自动等同宿主断开并退出UI”的隐式规则；stdin end/error和stdout错误按流契约报告，是否结束应用由应用生命周期策略处理。此为建议的默认语义，具体退出政策仍可在后续review确认，不把EOF误称为D12的页面Bridge断线。
- 示例、参数解析、类型与文档仅在整体review结束后的实现阶段同步调整；当前源码仍是旧--stdio/host机制，本轮未改代码。

### D21：Niva 自身日志与应用 stdio 完全分离

日期：2026-09-25。用户明确Niva自己的日志应走日志系统以避免干扰stdio。状态：已决策待实现。替代此前“框架日志统一走stderr”的建议；stderr也属于stdio。

- stdin/stdout/stderr作为应用与宿主的标准输入输出，由主窗口process API按既定范围使用。Niva框架不向其中混入启动消息、Bridge调试信息、资源错误或其他内部日志。
- 框架日志通过独立日志系统输出。建议默认落独立日志文件，具体目录、轮转/大小限制、级别及Devtools读取入口在实现前确认；可沿用应用UUID区分日志归属，不以展示名称作为唯一身份。
- 用户主动调用process.stdout.write/process.stderr.write仍按标准流语义输出，不因框架日志分离而被重定向。页面console及可选日志API的归属另行明确，不能静默把所有应用输出改成框架日志。
- 后续实现需一并审查当前print/println/eprintln、日志宏、错误退出/panic及依赖诊断路径，避免仅替换某个日志宏却仍有框架输出污染标准流。日志系统自身失败不默认回退写stdio；失败处理方式明确记录。
- API业务错误继续通过对应API通道返回；写日志不能代替reject/抛错/回调错误。日志中不得暴露token或其他秘密。
- 验收：框架启动、正常运行、API错误、断线、关闭等场景不向stdout/stderr混入内部日志；用户应用写入的字节保持原样；独立日志可定位相关错误并控制增长。

本轮仅整理review档案，未修改日志实现、配置、代码或运行验证。

#### D20 应用场景补充：Shell 脚本拥有 UI

用户指出普通Shell脚本也可通过标准流使用Niva UI。作为已定process stdio设计的使用场景记录：Shell负责原有命令/业务，Niva主窗口负责交互，双方通过双向管道交换应用消息。单向shell pipeline只提供一个方向，需要coproc、命名管道或其他明确的双向子进程连接方式。示例可选目录/开始/进度/取消等流程，当前不实现示例或引入额外host API。

### R12 / 第 1 轮：Devtools 的项目工作流

日期：2026-09-25。用户要求继续。主线程核查调试与构建入口，Luna只读核查项目导入/编辑/保存。仅review档案可写，不运行Devtools或构建。

**组成**：Devtools是使用Niva能力的React应用，AppModel管理当前项目、历史记录、语言/弹窗和多目标构建占用状态；ProjectModel管理项目路径、已读取配置与编辑草稿。UI通过这些model发起动作，不是独立的原生后台管理程序。当前源码仍大量使用Niva.api.fs/process/os，后续按D02/D17迁移统一Node风格接口。

**项目是什么**：项目目录及其niva.json是配置实体；导入普通前端项目时可以生成niva.json，前端源码/依赖仍属于该项目。debug.resource/entry和build.resource分别指开发资源/页面入口与打包资源目录，不能把“导入”理解成Devtools自动转换任意前端构建系统。

**编辑与动作的边界**：编辑器草稿与磁盘配置分开。保存/丢弃/取消需要在打开其他项目、重载、调试和构建前正确处理；用户选择丢弃后若重读失败，当前实现意图是保留草稿。构建依据需以已确认的磁盘/配置状态为准，不能用户看到未保存草稿却默默打旧配置。具体model证据由Luna补充。

**调试启动**：ProjectModel.debug先处理未保存状态，再检查资源目录，然后用当前Niva可执行文件启动另一个进程，传入配置/资源/入口等参数。当前为detached执行，并不会在此启动npm/Vite开发服务器。D04参数改名与D07调试资源代理尚未落地，应分别更新调用端与运行时。当前detached调试进程未来是否明确移交出Devtools会话所有权，需要与D12统一，不能自动保留例外。

**构建路径**：

- 原本机路径：ProjectModel.build根据宿主OS调用build-macos/build-windows，读取build.resource中的既有产物，复制/封装Niva执行文件，打包资源/图标/版本，随后按配置签名。
- 多目标路径：MultiTargetBuildPanel选择runtime kit、输出目录和目标，刷新配置后启动宿主平台的niva-packager，传manifest/config/resource-dir/output-dir/target参数；解析stdout结果JSON，逐目标显示成功/失败与路径/hash/签名等信息。
- 此处“构建”主要是桌面应用封装，不是自动编译用户React/Vue工程。前端构建和Niva原生runtime编译是另外的阶段，R13讨论统一职责。

**运行状态**：多目标构建用AppModel.packagerBuild持有当前项目，避免过程中的切换/重复操作；kit和输出目录路径保存在localStorage。GUI状态锁不等于磁盘资源快照，也不能阻止外部编辑器并发改文件；冻结输入需求留R13。

**自动化入口**：Devtools可读取--project/--build参数并自动打开/打包；这是Devtools的前端控制入口，不能与D04通用--config/--resource或已决定移除的--stdio混为一类参数。

**自举与API迁移**：Devtools本身也是Niva应用，重构后应使用fs/os/process/child_process等统一API及同源类型；实际启动子进程应归child_process，不能机械地把旧Niva.api.process.exec替换成Node process.exec。窗口/对话框仍用Niva专有接口。Vite开发配置当前固定3000端口并输出build目录；迁移时同时验证开发与打包产物的模块映射。

源码：packages/devtools/src/app.tsx、models/app.model.ts、models/project.model.ts、pages/project/multi-target-build.tsx、build-scripts/、vite.config.ts、niva.json。当前R12讲解中，未做GUI、保存持久化或打包运行验收。

#### AR-018：Devtools 自动构建失败仍依赖交互弹窗

- 归属：R12/R13；类别：自动化错误契约；状态：已确认待决策；优先级：P2。
- 证据：app.tsx的args.build分支调用project.build，失败时await modal.alert，成功才调用window.close。project.model.ts的传target路径注释期望可await确定结果、不弹成功提示，但最外层失败仍需要用户交互。
- 影响：无人值守调用若进入失败分支，不能从该路径保证自动结束及非零退出状态；静态结论，未实际触发。已有弹窗可能适合GUI路径，不应据此强行删除所有错误提示。
- 候选方向：区分交互构建与自动构建错误返回，后者有明确结果输出、非零退出与清理契约；下一章统一构建入口时决定最终方案。

R12 Luna补充：项目配置保存在项目目录niva.json；历史记录在Niva数据目录history.json，并按UUID或路径合并。当前validateConfig只检查name/uuid真值，不能代替Rust类型解析和完整字段校验。配置编辑器有字段和原始JSON模式，setContent即标脏；取消保存提示阻止动作。普通保存会写盘后refresh并更新历史，关闭中的保存走另一条路径。

#### AR-019：关闭项目时保存后历史元数据可能未刷新

- 归属：R12；类别：状态同步；状态：已确认待决策；优先级：P2。
- 证据：ProjectModel.dispose → resolveUnsavedChanges → persistEdits后，AppModel关闭当前项目；未经过普通保存后的refresh或HistoryModel.record。
- 影响：若关闭时保存了项目名称或图标，niva.json已写入，但history.json可能仍显示旧元数据；静态流程结论，未执行GUI复现。
- 候选方向：明确“保存成功后的项目元数据同步”为共同契约，覆盖普通保存及关闭保存，同时保留写入失败时的草稿。
- 未来验收：关闭时保存名称/图标后，历史列表与重开项目一致；取消/保存失败不丢草稿。

本轮luna_deep_worker（/root/architecture_inventory）交付导入/编辑/保存/历史流程核查；主线程负责调试与构建分工、D02迁移关联和章节整合。仅更新review档案，未修改实现或运行GUI验证。

### D22：打包统一为一个流程

日期：2026-09-25。用户明确“打包统一成一个流程，不搞两个流程”。状态：已决策待实现；本轮只记录，不改代码。

- Devtools图形界面与命令行共用一个打包入口和核心实现，单目标/本机平台只是目标选择的一种情况，不再分别维护“本机脚本打包”和“多目标打包”两套逻辑。
- 建议以现有niva-packager作为统一实现入口；R13逐项核对并补齐原流程需要保留的资源、图标、版本、平台封装及签名能力。用户已确定统一原则，具体接口与功能缺口在R13审查，不能因选现有入口就视为已经对等。
- Devtools负责项目配置、目标/输出选择、进度与结果展示；资源收集、格式生成、平台封装、校验及相关签名编排集中于统一打包流程，不在GUI再复制这些实现。
- CLI与GUI共享输入规则、错误、退出/结果契约；每目标结果可以不同，但不是不同打包架构。自动化失败不再依赖GUI弹窗，关联AR-018。
- 原build-macos/build-windows等独立流程在实现阶段迁移所需能力后移除，不增加旧路径fallback或兼容开关。平台专属模块可以作为统一核心内部实现存在，并不要求消除平台差异。
- Niva自身/Devtools自举打包也应复用同一核心；Rust runtime编译与用户前端编译仍是产物准备阶段，不能误认为统一打包就要把全部编译器塞入GUI。
- 验收：同一输入、目标和runtime通过GUI/CLI进入同一核心，结果元数据/资源布局与错误行为一致；覆盖单目标、多目标、部分失败、签名及既有使用流程；旧独立打包实现不再是活动入口。

当前源码仍有两条路径，本轮没有删除、替换、构建或运行任何实现。

### R13 / 第 1 轮：统一打包的输入、流程与交付边界

日期：2026-09-25。按D22统一打包决定推进。主线程核查kit生产和资源流程，Luna核查校验/结果/签名差距。只读源码、更新本档；未运行任何构建或打包命令。

**三种产物/角色分开**：

| 对象 | 包含内容/职责 |
|---|---|
| Niva runtime | 在目标平台运行的原生主程序，含内嵌页面运行时/NodeCompat及其索引 |
| build kit | 在宿主电脑执行的packager、各目标预编译runtime、manifest/哈希及许可材料 |
| 业务应用产物 | 选定runtime + 用户niva.json/资源/图标/版本等平台封装 |

这里跨平台打包主要是组装预编译runtime，不等于让终端用户安装Rust再交叉编译。packager较重的签名/PE/Mach-O处理依赖属于构建工具，不应自动计入运行时体积。小于3,300,000 bytes的现有门禁针对包含已确定Native与内嵌JS等开销的完整release主程序，不把build kit、应用资源包或只量裸库混为同一指标。

**统一流程建议**：

1. 接收项目配置、资源目录、目标列表、runtime manifest和输出目录；GUI/CLI共用相同输入契约。
2. 校验配置/名称/目标及runtime清单；资源目录采用同一套收集规则。
3. 读取明确指定的niva.json，准备业务资源索引与压缩数据；选择目标runtime。
4. 平台模块生成Windows EXE或macOS app结构，并处理图标/版本等平台元数据。
5. 统一编排校验、所选签名/公证流程与最终归档；完成最终产物后再确定hash和结果。
6. 输出逐目标状态和机器可读报告；失败有非零退出码，GUI只展示此共同结果。

当前核心实现已经具备的部分：lib.rs检查schema、packager/manifest声明版本、runtime声明版本、SHA-256、目标PE/Mach-O架构；prepare只收集普通文件、拒绝符号链接/异常相对路径，显式配置覆盖资源目录里的同名niva.json。业务资源prepare一次后供所有目标共享，但准备期间的磁盘读取不是源目录原子快照，图标及runtime等输入也仍需完整冻结策略，不能宣称UI禁编辑就保证可复现。

当前目标：windows-x86_64、macos-aarch64、macos-x86_64。Windows输出.exe，macOS输出包含.app的.zip；归档保存可执行权限。平台后端虽不同，属于同一个核心流程，不是保留两套用户流程。

**输出与失败**：当前每目标在输出目录内的临时目录组装，最后通过不覆盖已有目标的发布步骤交付；同名目标拒绝覆盖。某目标失败继续记录其他目标，成功产物保留；任一目标失败CLI退出1。全局预检失败给空results与顶层error。这是逐目标成功/失败语义，不是所有目标全成或全回滚。

**版本与来源**：runtime哈希证明读到的文件与清单匹配，架构检查防止拿错平台；版本字段比较不能单独证明二进制发布来源可信，kit供应链与发布证据仍在R14核对。当前CI配置先在原生runner构建各runtime，再汇总为宿主kit，只上传Actions artifacts；存在workflow不等于本轮已验证远端成功或正式发布。

**签名差距**：当前packager对Windows标unsigned，对macOS做ad-hoc；并不自动执行旧sign.macos/sign.windows配置。旧Devtools实现含macOS codesign/notarytool/stapler及Windows signtool入口。按D22应在统一核心内明确需要保留的正式签名/公证阶段、凭据来源和宿主平台条件，不让GUI另外维护一套独立签名业务。ad-hoc不等于开发者身份签名或公证，打包成功也不等于另一台机器能直接启动。

D03/D18的统一runtime、CommonJS/ESM注入开关也必须随完整runtime版本进入kit；项目配置透传、资源保留命名空间、运行时读取与公开类型要一起核对。不能混入不同版本的JS接口层来弥补旧runtime缺功能。

源码：crates/niva_packager/src/{lib,main,resources,macos,windows,archive}.rs；scripts/create-packager-kit.py；.github/workflows/packager.yml；packages/devtools/src/build-scripts/sign-*.ts。R13讲解中，实际产物/签名/平台启动本轮均未验证。

#### AR-020：统一打包前需要补齐正式签名/公证职责

- 归属：R13；类别：统一架构迁移差距；状态：已确认待决策；优先级：P2，最终发布用途另行定级。
- 证据：niva_packager::build目前按目标写unsigned/ad-hoc；旧sign-macos.ts与sign-windows.ts另行执行平台签名工具及macOS公证。
- 影响：直接切换到packager并移除旧路径可能丢失现有签名入口，不能把ad-hoc状态描述成正式分发就绪。
- 后续决策：明确支持的签名身份/平台组合，把所需能力纳入同一编排和结果契约；凭据不写入项目配置或仓库。最终归档与hash应对应签名后的实际交付产物。
- 验收：GUI与CLI相同签名输入得到相同阶段/错误语义；未签名、ad-hoc、身份签名、公证状态准确标记；目标机真实启动单独留证。

R13 Luna补充：macOS后端使用默认SigningSettings做ad-hoc并核验Mach-O；Windows修改PE资源后清除失效Authenticode指针，不能沿用模板原签名。架构验证范围为Windows x86_64及thin Mach-O x86_64/arm64；不将其外推到未列目标。Luna确认的主要迁移差距与AR-020一致。

本轮luna_deep_worker（/root/architecture_inventory）交付manifest/hash/架构、逐目标报告和签名核查；主线程整合kit生产、资源准备、D22单流程及发布边界。仅更新review档案，未修改代码或生成产物。

### D23 候选方案：目录资源布局与轻量安装器

> 后续按D27收敛：目录资源/大媒体读取方向保留；轻量安装器暂缓，不进入默认实施清单。当前交付主线为单EXE和绿色ZIP，MSI/WiX路线已取消。

日期：2026-09-25。用户提出大应用/大量媒体不适合单文件，可增加轻量安装器以缓解内嵌压缩问题。评审结论：可行，建议纳入统一打包的布局/交付选项；具体配置和安装器技术尚未决定。本轮仅记录，不实现。

**先区分两层**：资源布局解决运行时如何存储和读取大文件；安装器解决安装位置、快捷方式、卸载等部署事务。仅用安装器包住原来的整包内嵌压缩EXE，并不能解决运行时全量解压和内存开销。

| 资源布局/交付方式 | 适用性 |
|---|---|
| 内嵌业务资源 | 小型、资源较少的应用；保留现有便利性。Windows可表现为单EXE，macOS本来就是.app目录结构，不笼统称跨平台物理单文件 |
| runtime + 外置资源目录 | 大媒体或大资源项目，文件按需读取；可以直接以目录/归档作为便携应用交付 |
| 上述目录应用再封装安装器 | 需要安装/卸载体验的项目；安装器仍属于同一打包核心的末端封装，不新增独立业务打包流程 |

**统一流程建议**：使用同一资源清单、配置校验和runtime选择，根据布局把业务资源嵌入或复制到受控目录，然后执行平台元数据/签名与所选归档/安装器输出。GUI与CLI仍共用D22的一条核心；平台安装后端可不同。尚未选定安装工具，不自行实现一套安装引擎或引入新依赖。

**运行时建议**：沿用ResourceManager资源抽象，目录布局在普通应用启动中也是正式资源后端，而非只允许debug使用；与D04通用resource接口衔接。页面仍使用稳定应用origin和相对资源URL，业务不需要知道媒体位于EXE外还是.app资源目录内。Niva内嵌API runtime与用户的大媒体资产分开，外置业务资源不等于要求用户自己部署另一份JS runtime。

**大文件必须同时解决读取方式**：

- 媒体不放入当前单一Deflate资源数据块，避免启动时解压整包；目录后端按需打开/定位/读取。
- 当前custom_protocol.rs是GET-only、完整Vec响应和32MiB单响应上限。仅换成目录后端仍可能一次读整文件或被限制拒绝，需要一起设计HEAD、Range、206/416、Content-Length/Content-Range以及取消/有界读取。
- WebView自定义协议响应能力和媒体请求模式要在macOS/Windows实测，不把底层文件seek可用等同于视频拖动、连续播放、大文件内存占用已验收。
- 安装包运输时是否压缩与运行时资源是否压缩分开：可以安装时一次解包，运行时直接读普通媒体文件。媒体具体压缩策略/混合布局待定，不预先增加复杂配置。

“轻量”指runtime/安装引导自身和运行开销可控；包含全部媒体的离线安装包总大小仍受素材大小支配，不会因使用安装器而凭空缩小。在线下载型安装器是另一项分发能力，本轮未要求，也未纳入默认方案。

**未来验收/讨论**：大文件播放与seek、峰值内存、冷启动、路径根边界、缺失/损坏资源诊断；安装卸载与签名/平台启动；应用数据和独立日志按应用UUID放在数据区，不默认写安装目录，卸载是否保留用户数据需明确。业务资源外置不改变API授权或D12资源释放规则。

资料依据：当前源码custom_protocol.rs::response_for_request及R03已核的整包解压行为；Range用途及响应语义 https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/Range_requests 。本轮未生成安装包或修改代码/配置。

### D24：Windows 提供单文件与安装器两种交付，安装机制选型

日期：2026-09-25。用户明确Windows应能选择单文件或安装器，并询问自制Niva安装器与MSI哪个更好。Windows两种交付方向记为确定；安装引擎选型以下为评审推荐，尚未由用户最终拍板。本轮只记录。

| 交付选项 | 应用布局与用途 |
|---|---|
| 单文件EXE | 直接运行、资源内嵌，适合较小应用 |
| 安装器 | 将runtime与资源目录安装到目标位置，大媒体安装后直接按需读取 |

两者仍由D22统一packager入口生成，共享runtime/配置/资源清单及校验；安装器是输出封装选项，不重建一套项目构建流程。

**推荐：采用MSI/Windows Installer机制，不自研安装引擎。** MSI是Windows标准安装包/系统服务机制；WiX等是用于生成它的工具，不应把WiX误称为Microsoft内置工具。Niva负责从项目配置和资源清单生成安装描述并编排所选成熟工具，不要求应用作者手写安装脚本。

理由与责任分配：

- Windows Installer提供安装、维护/修复、卸载及失败回滚等基础机制，避免自己维护文件占用、权限、升级和卸载状态。具体组件、版本、升级与回滚规则仍必须正确编写，尤其自定义动作并非天然可回滚。
- 自制Niva安装器即使能复制文件，也需要完整实现安装状态、卸载、升级冲突、恢复等系统集成；该成本与框架核心目标不成比例。
- Niva UI依赖WebView2，而安装程序可能首先需要部署WebView2；自制WebView安装界面会引入先有WebView才能显示安装UI的启动依赖。应优先考虑标准安装UI；品牌、图标与文案不要求自研安装引擎。
- 如需自动部署WebView2等前置条件，可采用成熟bootstrapper编排官方runtime installer与MSI；即使最终外层是Setup.exe，底层仍可为标准安装机制，并不等于自研Niva安装引擎。在线/离线前置条件方案后续选定，不无条件下载或捆绑。

**大资源策略**：MSI可以使用内嵌CAB、外置CAB、未压缩来源或混合来源；安装后生成普通资源目录。大媒体不要求全部挤进一个MSI文件，也不应重新塞回应用EXE；需按最终安装工具的文件/CAB/包规模约束验证。离线总交付大小仍包括全部资源，安装压缩与运行时按需读取是两个问题。

**尚待确定**：具体MSI authoring工具/版本及许可条件，宿主平台制包能力（不能假设macOS上的现有Rust packager可直接运行WiX制MSI），安装范围/目录、快捷方式、WebView2依赖策略、升级/修复/卸载、用户数据保留与签名。应用UUID可作为稳定产品身份依据，但MSI ProductCode/UpgradeCode等不同标识的版本规则需正确设计，不能把所有GUID简单设成同一值。

**跨平台约束**：统一入口不等于每个产物后端均能在任意宿主原生执行。若所选工具要求Windows，应明确标注Windows制包要求或评估能满足跨宿主目标的成熟工具，不隐式添加远程构建服务，也不为追求形式统一而自写MSI引擎。本轮不更改既有跨平台打包承诺，待工具能力核实后再做取舍。

来源（2026-09-25核查）：

- Windows Installer回滚与自定义动作边界：https://learn.microsoft.com/en-us/windows/win32/msi/rollback-installation
- MSI压缩/未压缩资源：https://learn.microsoft.com/en-us/windows/win32/msi/compressed-and-uncompressed-sources
- 内外置CAB及大文件分卷：https://learn.microsoft.com/en-us/windows/win32/msi/cabinet-files
- WebView2部署与在线/离线安装器：https://learn.microsoft.com/microsoft-edge/webview2/concepts/distribution
- WiX工具定位：https://docs.firegiant.com/wix/

本轮没有安装工具、生成MSI/EXE或修改源码；仅更新review档案。

D24 Luna核查补充：当前WiX官方Media schema支持外置CAB（EmbedCab控制是否内嵌），可作为大资源分离依据。未找到足以确认现代WiX原生macOS/Linux宿主制包的明确支持声明，宿主能力继续标待核；不以Wine或.NET跨平台本身推导支持。WiX v3的.NET Framework要求与历史CAB规模案例不直接外推到现代版本。资料：https://docs.firegiant.com/wix/schema/wxs/media/ 、https://docs.firegiant.com/wix/tools/msbuild/ 。luna_deep_worker（/root/architecture_inventory）只读核查，主线程整合MSI职责与选型建议，无安装或执行。

#### D24 跨宿主要求下的选型修订：优先评估 NSIS

日期：2026-09-25。用户指出采用MSI工具可能妨碍macOS本地打包Windows安装器。结论：macOS可以生成Windows安装器；不能把具体WiX/MSI工具链的宿主限制等同于Windows安装器都不能跨宿主生成。

- 官方NSIS教程明确支持在Linux/BSD/macOS上编译Windows安装器。NSIS输出Setup.exe安装程序，可由macOS宿主的makensis组合预编译Windows Niva runtime、配置和资源；这一步不要求重新编译Windows Rust主程序。
- Tauri官方文档区分WiX生成MSI与NSIS生成setup.exe，并明确提供macOS/Linux到Windows的NSIS路线，作为成熟跨宿主实现参照；不因此引入Tauri运行时或直接宣称Niva已有该能力。
- 针对Niva已要求的跨平台打包体验，修正上一轮优先MSI的建议：优先评估NSIS成熟安装引擎，保持Windows两种交付——便携单文件.exe，以及安装器-setup.exe（安装后runtime+资源目录）。不是把两个同后缀文件都称为单文件运行模式。
- 若必须输出.msi，制包工具仍需单独评估；常见WiX路线不能直接假设macOS原生可用。此轮不承诺所有MSI工具都绝对不支持macOS，也不为保留MSI自行重写安装引擎。
- NSIS与MSI的安装状态/修复/回滚模型不同，不能将此前MSI提供的事务保证照搬成NSIS自动具有；统一packager要明确安装、升级、卸载和失败清理契约。
- 大媒体依旧采用外置payload/资源布局；NSIS官方文档列有约2GB的内嵌安装EXE限制，并建议大数据独立存放。后续锁定具体NSIS版本/插件和单文件规模后验证，不能承诺任意大媒体都塞进一个setup.exe。不要为绕过限制无验证引入第三方大包分支。
- 签名和Windows目标机安装/启动验收仍独立；macOS成功生成文件不等于Windows安装卸载流程已通过。

资料：https://nsis.sourceforge.io/Docs/Chapter2.html 、https://v2.tauri.app/distribute/windows-installer/ 、https://nsis.sourceforge.io/I_get_an_error_when_compiling_large_installers 。本轮只记录修订推荐，NSIS尚未由用户最终确定，也未安装工具、生成安装器或修改代码。

### D25：Windows 安装器仅在 Windows 宿主构建

日期：2026-09-25。用户提出只在Windows下制作安装器。按该范围收敛交付方案；替代D24中为了macOS本地制Windows安装器而优先评估NSIS的前提。本轮仅记录，不实现。

| 宿主 | Windows便携单文件EXE | Windows安装器 |
|---|---|---|
| Windows | 支持目标 | 支持目标 |
| macOS | 保留现有跨平台打包目标 | 不支持本机制包，明确提示需Windows |

- 安装器限定Windows宿主后，推荐回到MSI/Windows Installer路线；具体制包工具和版本（如WiX）在实现前核定。用户此轮确定的是宿主范围，不把它冒写成已锁定某个工具/版本。
- 不自研Niva安装引擎，也不因Mac无法制安装器而自动引入Wine、虚拟机或远程构建服务；用户可以在Windows使用相同项目与统一CLI/Devtools入口制作安装器。
- 保留D22单一打包核心：项目/资源准备、runtime校验和平台布局共享；最终MSI封装为Windows可用的后端阶段。宿主能力差异不是维护两套项目构建流程。
- Windows安装器采用runtime与外置资源布局以服务大媒体，便携模式保持单EXE选项。安装压缩与运行时资源按需读取分开，仍需D23的媒体读取验收。
- GUI根据宿主明确展示/禁用不可用选项；CLI对macOS请求Windows安装器返回清晰不支持错误，不静默改成便携EXE或省略目标。Windows单文件的跨宿主能力不因该决定删除。
- 签名、升级卸载、用户数据保留和安装器工具链的具体要求继续按R13/R14确认；生成安装包不等于目标机安装/运行已验收。

本轮只追加review决定，未修改代码、配置、安装工具或执行制包。

#### D25 Windows MSI 制作工具候选

日期：2026-09-25。用户询问Windows下制作MSI的工具。官方资料核查后记录以下候选，尚未选定/安装：

| 工具 | 方式 | 对Niva统一打包的适配判断 |
|---|---|---|
| WiX Toolset | 声明式安装描述、CLI/MSBuild生成MSI | 自动化集成优先候选；Niva生成描述并调用工具，用户不手写配置。具体版本及使用/再分发条件需核定 |
| Advanced Installer | 图形界面及CLI，提供freeware基础功能与付费版本 | 手工配置方便；自动化也可行，但所需功能和构建工具分发条件按版本确认 |
| Microsoft Visual Studio Installer Projects | Visual Studio扩展，Setup Project生成MSI | 微软提供的可视化选项，偏Visual Studio工作流，不优先作为轻量独立build kit的底层依赖 |

WiX不是无条件免费工具的同义词：官方当前说明自v6引入Open Source Maintenance Fee，并对适用组织列有收入条件；v7提供EULA接受机制。此处只记录官方规则存在，不代用户判断商业适用性、不接受EULA、不购买或绕过费用。选型时同时确认我们集成/分发工具与下游用户使用的实际条款。

官方来源：https://docs.firegiant.com/wix/ 、https://docs.firegiant.com/wix/osmf/ 、https://www.advancedinstaller.com/advanced-installer-free.html 、https://learn.microsoft.com/en-us/visualstudio/deployment/installer-projects-net-core?view=visualstudio 。技术倾向WiX自动化，最终工具/版本待确定；NSIS输出setup.exe不属于此处MSI工具列表。本轮只更新档案。

#### D25 WiX 对开源项目的使用条件补充

日期：2026-09-25。用户询问Niva作为开源软件使用WiX是否需要付费。本轮查阅WiX官方仓库当前OSMFEULA.txt及LICENSE.TXT，细化此前只提示维护费的笼统说明；不代用户接受条款或确定其收入/组织身份。

- 开源身份本身不是自动豁免条件。当前OSMF协议的适用段落针对使用官方Binary Release参与创收活动、且使用者年总收入至少US$10,000的情形；不足US$10,000列有豁免。不能只用“Niva免费下载”推断公司或下游使用者均豁免。
- 同一协议明确：它只适用于项目提供的官方预编译发行版，不限制按开源许可证取得源码、自行编译或分发自行编译的二进制；第4节允许独立编译而不受该EULA约束，但仍须遵守源码许可证。这是文本明确允许的路径，不是绕过技术或费用检查。
- 当前源码许可证为MS-RL。分发包含WiX代码的文件需提供相应源码及许可证并保留声明；完全自有且不包含其代码的文件可以采用自己的许可。Niva整合build kit时应按实际分发的WiX组件/附属依赖逐项满足义务，不能只标一个“开源”就忽略材料。
- 对Niva可评估两种合规集成：使用满足相应条件的官方工具，或自行构建固定WiX源码版本并随build kit提供必要许可/源码材料。具体版本及其附带条款需要冻结核对，不将当前main条款自动套用到所有旧版。
- 不因Niva开源而自动给所有下游商业用户作免费承诺；也不将官方发行版维护费笼统说成使用WiX源码必然收费。MSI制作机制的技术推荐保持，选用发行版/分发方式尚未确定。

一手依据：https://github.com/wixtoolset/wix/blob/main/OSMFEULA.txt （适用范围及第4节）；https://github.com/wixtoolset/wix/blob/main/LICENSE.TXT （MS-RL）；https://docs.firegiant.com/wix/osmf/ 。本轮仅条款信息核对与review档案补充，未接受EULA、购买、下载工具、编译或修改代码。

#### D25 WiX 条款记录确认

日期：2026-09-25。用户要求记录后继续。保留官方预编译发行版适用条件、按许可证自行构建/分发固定版本这两条路径；尚未选定版本或代用户接受任何条款。后续选型按冻结版本的实际许可证和分发内容核对。

### R14 / 第 1 轮：依赖、CI 与发布门禁

日期：2026-09-25。用户要求推进下一章。主线程核查仓库门禁、依赖审计快照和许可打包脚本；Luna只读核查CI、hook和kit检查脚本。本轮不运行测试、远端CI、安装或发布。

**依赖分三层管理**：

| 层 | 示例 | 需要证明 |
|---|---|---|
| 随runtime交付 | Rust Native与内嵌JS/vendor | 固定版本/来源、对应许可、功能及完整runtime体积 |
| 构建工具 | packager、签名工具、未来MSI工具 | 构建宿主条件、版本/来源、使用与再分发条件；不要混入应用runtime大小 |
| 目标系统环境 | 系统WebView、Windows WebView2等 | 目标机可用性、安装/升级路径及实际运行表现 |

现有dependency-audit.md已标为历史升级快照，不能引用其“最新/健康/审计通过”来证明当前依赖状态。未来实施后的锁文件/工具链、JS生成产物和Native构建必须对应同一版本；本轮不执行依赖更新或漏洞扫描。

**检查与验收的职责**：Rust fmt/check/clippy/test、TS类型与Vite构建等检查源代码；协议/API测试检查行为；WebView和原生UI真机测试检查平台协作；安装/签名/下载后启动检查交付。各层不能互相替代，R10的真实项目也不是全部平台发布门禁。

仓库AGENTS要求的检查以其当前内容为准，包含Windows条件代码的target check和Devtools直接build。pre-commit不是可靠的TS通过证据；CI文件存在与对应提交实际跑绿也要分开。本轮只核配置，不声称GitHub工作流已经成功。

**产物身份建议**：未来每个候选记录源码commit及明确输入清单、依赖锁/工具链、目标架构、runtime版本与hash、应用/安装器hash、字节数、签名/公证状态、CI运行和目标机验证记录。最终签名/封装后hash要重新对应最终交付文件；不能测试A二进制却发布B二进制并共用一份“通过”。这不要求本review处理其他session的工作区，而是未来发布候选冻结的规则。

**体积与构建策略**：当前release profile使用LTO、opt-level=z、codegen-units=1、strip和panic=abort。它们偏向紧凑产物，但panic=abort不执行正常栈展开清理，不能用正常断线释放契约推断崩溃时所有Drop或子进程清理都会执行。当前严格小于3,300,000 bytes门禁仍需对全部确定功能的完整release主程序按平台复测；新require加载器、资源后端等变更不能引用旧体积数字。

**分发与许可**：create-packager-kit.py目前收集runtime/packager的Rust依赖许可、NodeCompat vendor notices与源码链接，并生成hash清单。它只证明有这条材料生成流程；最终应用、MSI、kit各自包含哪些第三方内容应分别核对。WiX等新工具的具体许可也不因Niva开源自动略过。

**公开文档**：统一runtime/types、API表、Bridge协议、参数、两个注入开关、process stdio、日志、单EXE/MSI等新设计落地后，需对照同一实现更新文档与网站。当前review决定不能直接变成“已支持”或v1.0门禁已关闭；README/站点旧体积对比及跨平台声明必须有相应证据。

**未来v1.0收口**：仓库现有origin权限、资源鉴权/路径、原生桌面行为、真实CI、Windows真机等门禁逐项用对应提交/运行记录关闭。仅勾选文档、历史report或target check不足以关闭门禁；已确定的新设计也需完成对应验收。本轮不判定所有门禁当前是否通过。

#### AR-021：最终业务产物的第三方许可材料传递需核实

- 归属：R13/R14；类别：分发证据缺口；状态：待核实；优先级：P2。
- 证据：create-packager-kit.py为kit收集许可，check-packager-kit.py检查相关文件；本轮对packager的lib/macos/windows/resources模块定向搜索未发现相应LICENSE/NOTICE自动附带逻辑。
- 影响：不能从kit具备许可材料推断下游单EXE/app/MSI同样附带完整材料；不据此直接作法律违规结论。
- 后续：按实际分发组件逐项确定材料及交付位置，统一打包自动带出所需内容，固定版本核验；当前不修改原文件或产物。

当前R14讲解中。全部为静态核查与档案整理，后续实际实施/测试需要等整体review收口及用户明确指示。

R14 Luna CI核查补充：ci.yml配置macOS/Windows的Rust fmt/check/Clippy/workspace tests、NodeCompat测试与Devtools build；Native upstream suite仅macOS；两平台构建后检查Niva主程序严格小于3,300,000 bytes，Ubuntu另跑网站typecheck/build。packager.yml配置三个原生目标/宿主的runtime与packager构建、packager crate测试、kit组装/检查和ZIP上传，但未在该产物流水线设置同样体积门禁。

check-packager-kit.py检查ZIP路径、manifest hash与许可材料，调用打包器生成三目标fixture，核对报告/hash及macOS执行位；在macOS还解包验证ad-hoc签名。不安装、不启动目标应用。pre-commit的TS命令失败后有|| echo，因此不能视为提交类型门禁。上述配置已读，远端执行结果本轮未查。

#### AR-022：kit 产物流水线未绑定完整的 runtime 体积门禁

- 归属：R14；类别：发布门禁缺口；状态：已确认待决策；优先级：P2。
- 证据：ci.yml检查其构建的release主程序大小；packager.yml另行产生Windows x64/macOS arm64/macOS Intel runtime并组装kit，没有对应逐产物体积检查。
- 影响：普通CI中的体积结果不能直接证明发布kit内每一架构的实际runtime都满足3.3MB门禁，尤其需要避免不同构建/架构结果混用；不宣称现有产物已超限。
- 后续：对最终进入kit的各runtime文件按hash绑定体积、平台/架构及功能范围证据，统一检查门禁，保留失败结果而非静默上传为可发布产物。

本轮luna_deep_worker（/root/architecture_inventory）交付CI/hook/kit检查边界；主线程整合依赖分层、产物身份和AR-021/022。没有执行检查或修改CI/代码。

### D26：WiX 采用自行编译版本

> 后续已被D27替代：当前不再集成MSI/WiX，此段及体积预算保留为历史讨论，不进入实现清单。

日期：2026-09-25。用户明确“我们直接用自己编译版本”。状态：已决策待实现；仅记录，不开始下载/编译/安装。

- Windows MSI工具后端采用从WiX源码自行构建的版本，不再把官方预编译发行版作为默认集成来源。沿用D25：MSI只在Windows宿主生成，Mac仍可生成Windows便携EXE。
- 后续固定WiX tag/commit、源码和依赖校验信息、构建工具链及可复现命令；具体版本尚未选择。对锁定版本核对其附带许可证，不能把当前main条款自动外推到任意版本。
- 分发自编译WiX时附带所需对应源码、MS-RL文本、版权/归属声明及相关依赖材料；自编译不等于免除开源许可证义务。使用与分发按此前核查的官方条款执行，不代用户接受官方预编译版EULA。
- 预先构建并验证WiX工具，计划作为Windows build kit中的构建依赖交付；不要求Niva最终应用用户编译WiX，也不把WiX链接进Niva runtime主程序。具体附带文件布局在统一打包实现阶段确定。
- Niva统一packager生成安装描述、调用该固定WiX工具产出MSI；不自研安装引擎，不增加另一套GUI独立打包逻辑。
- 构建成功、工具可运行、MSI结构正确、Windows安装/升级/卸载与签名验证分别留证，不因“自行编译”就省略任一验收。

此决定替代前文“官方发行版或自行编译尚待选择”的分支；未决项只保留具体版本、构建/分发材料及安装行为配置。当前仍处review阶段，没有代码或配置改动、没有执行编译。

#### D26 自编译 WiX 纳入 Windows build kit 的体积预算

日期：2026-09-25。用户询问增加体积。本轮只查询公开发行元数据、NuGet页面及源码构建目标，没有下载/运行官方工具或自行编译；以下是参考估算，不是Niva自编译实测，WiX最终版本尚未冻结。

公开参考（WiX 7.0.0，保留发布页面显示单位）：

| 参考物 | 压缩下载体积 | 范围 |
|---|---|---|
| wix NuGet CLI包 | 约4.37 MB | .NET工具包，不包含完整独立.NET运行环境 |
| WixToolset.UI.wixext | 约760.03 KB | 标准MSI界面扩展 |
| WixToolset.Util.wixext | 约902.87 KB | 可选实用扩展 |
| 官方wix-cli-x64.msi | 约11.8 MB | 官方CLI安装包，仅作大小量级参照，不直接拿来作为自编译结果 |
| 官方artifacts.zip | 约180 MB | 全量发布构件集合，不是应该全部塞入Niva kit的最小工具集 |

**暂定工程预算**：在复用构建机已具备兼容.NET环境、不附带完整.NET运行时及完整源码归档的条件下，仅自编译WiX CLI/必要依赖/少量扩展的Windows kit压缩增量，先按约10–20 MB预留。依据为上面的工具包/官方CLI量级并留打包方式余量，置信度中低；实际自编译可因目标framework、架构、扩展及符号/压缩设置变化。

此数不是“完全自包含kit”的总增量。独立.NET运行环境（若携带）、按许可交付的对应源码归档，以及其他新安装器工具必须单列；这些附加项本轮未取得完整可靠体积，不虚构总数。解压占用也未实测，后续必须同时记录压缩包增量与解压字节数。

WiX v7.0.0 wix.csproj声明net8.0/net472目标，可研究依赖系统已有.NET Framework的构建与携带运行时的自包含方案；不能仅看到target就保证某份最小文件集在干净Windows机器上可执行。无需给kit附带整个.NET SDK或Visual Studio来编译用户MSI这一目标，仍要通过实际工具启动/制包验证。

材料交付建议：二进制工具文件集与对应源码材料分别清楚列出；对应源码可以作为同版本单独交付物的方案再按固定版许可核对，不把“估算没包含源码”误写成不再履行D26义务。WiX只增加Windows build kit，不进入Niva runtime主程序或最终应用执行依赖，Mac kit不因不能制MSI而默认带Windows工具。

来源：https://www.nuget.org/packages/wix/7.0.0 、https://www.nuget.org/packages/WixToolset.UI.wixext/7.0.0 、https://www.nuget.org/packages/WixToolset.Util.wixext/7.0.0 、https://github.com/wixtoolset/wix/releases/expanded_assets/v7.0.0 、https://github.com/wixtoolset/wix/blob/v7.0.0/src/wix/wix/wix.csproj 。GitHub release API本轮限流，使用公开asset页面取得大小；.NET运行时HEAD查询失败，因此未报告其精确字节数。

当前仅体积预算记录，不改变3.3MB runtime门禁，不修改代码或生成产物。

### D27：收敛便携分发，Windows 启动时检查 WebView2

日期：2026-09-25。用户调整方向：只支持单文件应用、绿色ZIP或其他简易安装包；不再为工具包引入WiX，并提出Windows Niva主程序检测、缺失时安装/升级WebView2。分发范围变更记为确定；以下WebView2流程为评审建议，待细节确认。始终只review，不实施。

**覆盖此前决定**：D24–D26的MSI/WiX默认路线及自编译工具集成不再进入当前实现计划，相关资料/预算仅保留历史。D22统一打包继续有效。Windows优先单EXE与绿色ZIP（runtime+外置资源）；其他简易封装尚未选定，不据此新建安装引擎。macOS上制作Windows便携EXE/ZIP不受原MSI宿主限制影响。

**WebView2引导方案可靠性的条件**：

1. 在创建任何WebView之前，由Rust/Win32启动层检测当前用户可用的WebView2 Runtime及版本；不是只检查Edge浏览器。官方检测接口可能返回Edge预览通道，生产runtime判断需按所选部署策略核对。
2. 满足Niva所需最低版本时直接启动；缺失或低于最低版本才进入安装/升级流程，不把“不是网上最新版本”当作每次强制升级理由。具体最低版本依据Wry及所用特性确定，当前不随意指定数字。
3. 先用不依赖WebView的原生提示说明缺失环境、下载及安装，允许取消；启动阶段不能用尚不可用的Niva网页UI或Node API完成检测/安装。
4. 在线使用微软官方Evergreen Bootstrapper；离线可提供微软官方Standalone Installer作为独立附加文件。引导来源限定官方，执行前验证安装器发布者签名；不从任意远端拉取脚本以管理员执行。
5. 管理员权限不是所有安装的前提：微软安装器支持per-user与per-machine。建议优先普通用户安装，确需全机安装/更新已有机器级runtime时由Windows UAC提升安装器子进程；不把整个Niva应用永久提权，也不绕过用户/系统权限策略。用户提出的管理员脚本可作编排，实际安装仍交微软安装器。
6. 等待安装结束，检查结果并重新检测当前用户可见的runtime版本；达到要求后才创建WebView，必要时由普通权限应用重启。不能仅看到安装进程退出就宣称可用。
7. 用户取消、无网络/代理受限、企业策略阻止、安装失败、不支持的OS等场景给出明确退出/重试或手动安装路径，记录到D21独立日志；不无限循环下载或反复弹UAC。

Evergreen通常由自身更新机制维护，更新后新建WebView环境或重启应用才能采用新版本；Niva只保证启动兼容条件和清晰失败行为，不再实现另一个通用更新服务。多应用共享的WebView2 Runtime不随某个绿色应用删除而自动卸载。

**体积与便携边界**：建议启动时按需取得bootstrapper，不为了自检将完整WebView2或安装器塞入单EXE，避免影响3.3MB主程序目标。在线bootstrapper官方描述约2MB，但其后下载runtime；离线附加包较大，单独列明。绿色应用文件布局不等于系统完全零变更，缺WebView2时经用户同意部署共享依赖应明确告知。

**验证**：Windows普通用户/管理员、runtime已满足/缺失/过旧、在线/离线、安装取消/拒绝/失败、版本复检、普通权限重新启动，以及单EXE/ZIP两种布局。只检查版本通过不替代实际WebView创建与关键API验证。当前未运行任何安装脚本或变更系统。

官方依据：https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution （检测、官方在线/离线安装器、用户级/机器级安装及更新采用方式）。当前源码定向查看仅见webview2-com依赖，不能据此宣称上述引导已实现。本轮只更新review档案。

#### D27 方案确认：Windows 10/11 脚本在线安装 WebView2

日期：2026-09-25。用户要求记录“win10011用脚本线上安装”，按Windows 10/11说明后用户表示“好，继续”，该平台范围及在线引导路径记为已确认方案，当前不执行。

- Windows Niva在创建WebView前检测WebView2 Runtime；缺失或低于所需最低版本时，进入安装提示并调用安装脚本。
- 脚本在线获取并调用微软官方Evergreen Bootstrapper完成安装/升级，校验发布者签名；不自行部署Runtime文件、不使用非官方安装源。
- 当前采用在线方案，不将完整离线WebView2安装包或WiX工具纳入默认kit/单EXE。断网时明确提示需联网后重试，不静默切换未确定的离线方案。
- 安装按用户级/机器级需要执行，确需管理员权限时提升安装器子进程并走正常UAC；用户拒绝时停止本次安装。主Niva应用保持普通权限。
- 等待脚本/安装器结束后重新检测版本，满足要求才启动WebView；失败给出明确诊断并写入独立日志，不反复弹窗重试。
- 具体脚本内容、最低版本与原生提示实现留到整体review结束后；不在本次任务中创建脚本、下载执行程序或修改系统。

本条确认D27的在线选择，其余资源布局、统一打包、stdio与独立日志约定不变。

## R15 当前方案与待决项

更新日期：2026-09-25。当前进入全局回看，不表示review已全部确认或允许实现。下表汇总仍有效的决定；前面章节保留源码现状、讨论历史与证据。出现前后差异时按明确的后续用户决定理解，不执行已被替代方案。

### 当前方案总览

| 领域 | 当前确认的方向 | 关联 |
|---|---|---|
| Native与页面分层 | Rust负责系统能力/资源/鉴权，WebView执行页面；原生API执行与模块接口导出分开 | R01–R08 |
| 统一页面runtime | @niva/runtime集中维护TypeScript实现及注入；所有API实现在Niva对象上，CommonJS/ESM仅导出同一接口；@niva/types同源交付 | D01/D03/D13/D17 |
| 兼容环境开关 | CommonJS与ESM两个独立开关；基础Niva API不依赖开关。用户模块覆盖受尊重 | D18/AR-016 |
| CommonJS加载 | 内置模块直接注册Niva接口；用户JS/JSON模块经同步XHR请求Rust解析/读取，页面执行并管理缓存/循环；不支持.node | D14/D15/D17 |
| ESM | 浏览器标准import/import()；提供Node API的.mjs facade和import map，不自建ESM执行器 | D16 |
| 三种Bridge | WS异步/流/二进制；sync XHR同步；IPC fallback一次性异步JSON，具体API范围待复核 | D08/D09及API总表 |
| 静态与动态数据 | os.info按最新要求为静态数据；动态系统查询等按API表分类，不把纯JS或静态读取算作第四种Bridge | API总表及R04更正后续说明 |
| 资源生命周期 | 统一资源释放抽象；Bridge会话断开双端撤销资源、调用失败；重连不恢复/重放旧任务 | D10/D12 |
| 权限与联网 | 原生侧窗口/来源授权保留；获准HTTP可访问内外网及localhost；IPC降级不扩权 | D11/R05 |
| 启动与身份 | --resource/--config是正式接口；持久目录使用完整应用UUID，不随展示名变化 | D04/D05 |
| 宿主与日志 | 主窗口process stdio承接Shell/Python等宿主；移除api.host、--stdio和内置NDJSON。Niva框架日志独立于stdin/stdout/stderr | D20/D21 |
| Devtools | 使用统一Node风格API开发，Vite适配指向同一Niva实现；文件与子进程职责正确迁移 | D02/R12 |
| 统一打包 | GUI/CLI一个核心；Windows单文件EXE及绿色ZIP，目录资源用于大媒体；macOS保留平台app/归档形态。暂不集成MSI/WiX | D22/D23/D27 |
| Windows依赖引导 | Windows 10/11启动检查WebView2，缺失/过旧时通过脚本调用微软官方在线安装器，按需UAC，安装后复检 | D27 |
| 兼容验收 | Cypress Real World App为已选主项目，JS版tsc保留补测；固定上游契约测试与真实WebView/平台验收互补 | D19/R10/R14 |

### 已被替代或收敛的讨论

- “只有一个Node兼容注入开关”已由CommonJS/ESM两个开关替代。
- os.info每次走同步XHR的早期举例，已由用户后续静态数据要求替代；底层当前RPC可达性留在API表中作为现状。
- 禁止用户覆盖内置import映射的候选未采用；用户可覆盖，自行承担差异。
- api.host、--stdio、框架日志走stderr均不属于目标设计；由普通process流和独立日志系统替代。
- 旧本机/新跨平台两套打包流程应统一；单EXE/ZIP只是统一流程不同输出布局。
- 自研安装引擎、MSI/NSIS选型、WiX自编译与kit体积预算均保留为历史，不进入当前默认实现清单。若未来再要求简易安装包，另行确认具体封装，不能自动恢复WiX路线。
- http-server不再是主验收项目；已选Cypress Real World App。json-server、旧RealWorld、Hackathon Starter等保留为候选筛选记录。

### 需要继续共同确认的产品规则

| 待决项 | 已确定部分 | 下一步讨论 |
|---|---|---|
| IPC fallback逐项清单 | 文件/网络需有实用能力；一次性异步JSON；不做Native同步/对外流/原始二进制传输 | 按API总表分组确认具体方法、文本/元数据分支及例外；不能把表中历史“必做/建议”整体视为冻结 |
| 两个注入开关缺省值 | 相互独立，Niva命名空间始终按范围提供 | 建议两项默认关闭、用户显式开启；此为提案，待用户确认 |
| 调试入口 | --resource/--config正式化，已有三Bridge与明确报错边界 | D07开发服务器经自定义协议代理仍是候选；确定是否纳入本次实现，还是先保留直接开发URL路径 |
| 宿主/任务生命周期细节 | 无--stdio，D12断线清理资源 | 明确stdin EOF/BrokenPipe是否仅作为流事件；调试子进程等是否允许显式移交出会话Owner，不能默许detached例外 |
| 正式签名/公证范围 | 放弃MSI不等于放弃便携EXE及macOS应用签名 | 确认统一打包保留哪些身份签名/公证入口；不要因去掉安装器而顺便删除这些能力 |

### 技术核查项，不当作需要用户逐个命名的审批

- 统一配置字段名称、Node契约版本、WebView2最低版本和脚本细节，应根据源码/目标特性及明确产品规则形成具体可review方案；不随意给不可靠版本数字，也不为常规命名增加审批。
- require动态代码执行与CSP、ESM启动次序、Vite映射、平台资源Range/HEAD/大媒体内存、正式签名支持边界均需要具体验证；这些不是架构原则确定就已经实现。
- 当前源码与最终方案差异要由实施清单列出。CI、目标机、最终hash/体积和许可材料按R14门禁验证，不引用旧报告代替新产物。

### 批注收口规则

- AR-001是外部合并状态记录，本任务不处理，不阻断review；AR-016已接受用户覆盖行为，不再要求禁止覆盖。
- AR-002/003/006等随已定架构重构处理；AR-004/005已由用户明确要求修复。
- AR-009至AR-014涉及取消、排队和终态，其修复原则与D12一致；具体补丁仍等实现阶段。AR-015/017/018/019及发布/材料缺口按原条目留待总体排序，不因推进章节而关闭。
- AR-007代理方案、AR-008条件性竞态、AR-020签名职责、AR-021许可传递、AR-022产物门禁需明确方案/验证，不把所有批注一律称为已复现bug。
- 整体review尚未结束：先处理上述待决产品规则并回看未覆盖项，再整理实施顺序与验收条件。只有用户明确要求开始实现后才能修改代码。

本轮仅对既有review内容作归纳与状态更新，没有代码/配置/依赖变更，也没有执行测试、安装或构建。

R15只读一致性复核：luna_deep_worker（/root/architecture_inventory）核对决定覆盖链，主线程据此将D23安装器候选明确暂缓，保留目录媒体布局，并补入正式签名/公证待决项。许可材料与体积门禁归技术/验收工作，不再要求用户另选一个产品方向。进度栏已同步到R15。没有代码或配置改动。

## 实现阶段授权更新 — 2026-09-25

用户最新明确授权：fallback所需Rust接口由执行方自行决定，保证基本文件读写、HTTP/HTTPS和exec等高层能力，并开始规划落实全部有效决定。此前“只review”的工作阶段结束；旧记录保留为历史，不再阻止此次授权范围内实现。实施顺序、文件所有权、默认取舍与验收台账见`docs/architecture-implementation-plan.md`。MSI/WiX等被撤销方案不恢复。

### AR-023：取消Stream future时原生socket注册表缺少Drop兜底

实施阶段核查（2026-09-25）：`api_manager::run_job`会在超时/取消竞态中丢弃handler future；此前socket的`remove_active`只位于await之后，被丢弃时不会执行，UDP注册表持有的Arc和listener的待接收句柄可能残留。主线程在TCP/TLS、listener、UDP注册后加入RAII守卫，正常完成与future被丢弃均清理所属资源；同时保持Owner过滤，不能误清兄弟连接。已补“丢弃handler future后UDP端口可重新绑定、兄弟注册仍可用”的回归测试。当前rustfmt通过，测试执行等待统一runtime构建接通；不能提前标为验收通过。

### AR-024：Rust已有能力时JS不得另造一套实现

2026-09-25，用户补充并授权按此原则复核实现：Rust已实现的文件、进程、网络/TLS等能力以Native为实现来源，JS Node API只适配既有桥接契约；若Native尚缺Node要求的流、背压、取消或对象生命周期语义，应扩展Rust桥接，再由JS包装。不能因“Rust已有某个高层API”而把更丰富的Node语义降级成单次文本调用，也不能把重复协议实现藏在vendor包里。HTTP/HTTPS明确采用Rust ureq客户端；客户端与Node http.createServer是不同职责。具体逐项复核和测量记录在[实施台账](architecture-implementation-plan.md)。

2026-09-26执行结果：`http.requestText`及Node `http.request/get`共用Rust ureq；Node client流由Rust处理网络/TLS和逐块背压，JS只适配Node对象。`http.createServer`继续由JS处理协议并使用Native TCP/TLS。`fs.cp`无filter时一次Native递归复制；有filter时JS运行用户回调，Native完成实际文件复制。最初final release SHA `159b98...`的完整远端grant和trusted-debug IPC矩阵在前台bundle测试各24项通过；随后最终release加入stream header pair序列化修正，SHA `e0ba...`的NodeCompat真实WebView smoke通过178项，独立lease-only通过，但Mac锁定时完整IPC矩阵停在lease reply，待解锁后绑定最终SHA复跑。最终macOS ARM64 release实测2,974,288 bytes（实施台账有完整SHA）。Native仅不支持强制FICLONE，其余已知Node stream/Agent差异和平台边界记录在覆盖表。Windows真机及最低macOS 11均未实测。

### D19 后续验收范围更新 — 2026-09-25

用户表示Cypress Real World App依赖的ESM路径不能兼容时不强求，随后又取消替换验收项目的工作。维持D19既有选择，不新增json-server或其他项目，也不为了RWA增加`require(ESM)`；CJS-only边界按D14/D15继续有效。RWA因`require('dinero.js')`进入纯ESM包而未启动的结果保留为明确的范围外阻塞，不改上游代码、不减官方测试分母，并继续完成与该项目无关的Native、Bridge、平台和产物验收。

### AR-025：本地custom protocol origin按应用UUID隔离

2026-09-26。用户明确提出将应用UUID纳入Wry自定义协议名，使同一app重启后origin稳定，不同app拥有不同origin。macOS/Linux使用`niva-<32hex>://app`；按Wry 0.57规则，Windows/Android映射到`http://niva-<32hex>.app`。WebKit按origin隔离localStorage，因此不再依赖仅macOS 14+可用的`WKWebsiteDataStore`标识API。当前代码已把entry别名、静态资源处理、CSP、WebSocket、IPC来源归一化和`__niva_fs` CORS绑定到精确UUID origin；外部debug origin保持原策略。macOS 26.6.2固定bundle身份的`.app`三进程 smoke验证同UUID跨重启存储持久、异UUID隔离；裸debug进程未持久，不能代替产品`.app`证据。build脚本最低目标macOS 11.0，但该旧系统未真机验收；Windows缓存target check通过，无WebView2真机证据。不得删除或迁移现有默认store数据。此决定取代固定`niva://app`/`http://niva.app` origin以及此前等待macOS14以下策略的讨论。
