# Niva 架构 review 实施计划与验收台账

## 收尾交接：按用户额度要求暂停（2026-09-25）

用户要求尽快结束，停止新增调查、构建和全量测试。已通知全部子代理停止；保留工作分支`codex/architecture-implementation`全部未提交改动，无commit/push/发布。以下状态优先于后面的历史进度。

**已经实现并有对应验证快照**：统一TypeScript runtime/Niva命名空间、CJS/ESM独立开关、Rust CommonJS解析、三Bridge及有界IPC文件/HTTP/HTTPS/exec、资源取消、主窗stdio/独立日志、UUID目录、Devtools Node API迁移、统一packager和external资源、Windows WebView2引导代码、隔离类型包。

**已验证**：IPC远端授权24项；真实初始化5组；原版TypeScript CLI成功/失败2组；macOS两种打包布局各7项；stdio与退出子进程清理；runtime130/130；固定Node22.14官方JS48/48、Native10/10；Rust快照175+22+2+10项及fmt/check/clippy、Windows target check；types四种独立安装消费者、完整typecheck与Devtools build。各项对应具体快照，不能合并声称最新源码全量验收。

> 2026-09-26 路由决定更新（覆盖下文早期“IPC fallback”提案）：对外分为异步API与Node兼容同步API。普通异步调用固定走平台IPC/`evaluate_script`；大吞吐文件/网络、二进制和流式stdio在每个新操作创建时优先使用已建立的WS优化bridge，否则由同语义IPC Channel承载，IPC binary frame在边界Base64编码。资源及其后续control/signal操作沿用创建时bridge；已提交操作断连时明确失败，不重放或迁移。同步XHR只服务需要同步返回的Node兼容API，按公开方法首调warning；`process.chdir`为异步API。最新macOS arm64完整release SHA `9108b915…` 为2,994,936 bytes；其真实WebView基础bridge 7/7、可信IPC 23项、远端grant IPC 21项通过，两个IPC场景各跳过2项隐藏窗口lease心跳。Windows真机和完整资源owner生命周期仍待验收。

**最后构建**：暂停前已经启动的release构建正常完成，未再启动新构建。`target/release/niva`为macOS ARM64，3,106,128 bytes，SHA256 `6dba027b7a3198269be21957cdb933380764b6930e76bac24ad4b213e940cf57`；runtime完整构建fingerprint `fad96616b63d1de542452fe13dbb5a58c11833131f144e28db645727d54609ea`。该Native产物包含版本元数据调整，但尚未用它重跑RWA或全部Native验收。最近已经完成主要真实验证的冻结产物仍为`/tmp/niva-architecture-release-e269ac719067`。

**必须继续的事项（未完成，不宣称全部落地）**：

1. RWA真实后端尚未启动成功：最近失败为依赖访问`process.versions.node.split`。版本元数据已改、runtime130/130及JS48/48已过，最新Native也已构建，但RWA未复跑。接着运行`examples/realworld-acceptance/niva.py`，成功后运行已准备的`ui.py`；不得使用host Node替代后端。工作目录`/tmp/niva-realworld-79aa5b1/.niva-compiled`、原前端`/tmp/niva-realworld-79aa5b1/build`。
2. 明确debug-entry/token的IPC模式仍在90秒内无页面报告，`/tmp/niva-ipc-trusted-debug-v5/result.json`；普通远端精确授权IPC24项通过不能替代它。继续用`examples/ipc-fallback-smoke/run.py --trusted-debug`定位（建议补独立HTTP进度回报）。
3. macOS WebKit跨应用存储隔离未修：Wry 0.57.0忽略macOS的WebContext data_directory；当前macOS26.6.2可用`with_data_store_identifier(UUID字节)`，需要builder接入与跨UUID/同UUID改名真实验证。macOS<14会退到共享default store，旧系统策略须明确，不能静默共享或丢数据。此次调查无代码改动，不删除现有默认store。
4. DevtoolsCLI首轮缺kit自然失败、随后成功fixture写入测试kit；后续随机UUID读到该选择，暴露上述共享store。后续driver曾清/写`niva-devtools-packager-kit`，未备份原值，不能保证没有影响原默认store；已停止这种测试。最初`/tmp/niva-devtools-cli-check-e269ac719067/result.json`记录无seed原dist的missing-kit确实失败，说明后续污染很可能来自本次成功fixture，但不据此猜值恢复。只修隔离，不删除未知用户数据。
5. Windows真机SSH连接拒绝、当前Mac锁屏（系统状态确认），窗口焦点/菜单/拖动等完整桌面验收未完成。Windows安装脚本未执行、未触发UAC；交叉编译不是Windows运行通过。
6. OPT-01混合代码压缩/媒体按需存储仍未开始；external ZIP已实现但不代替此最后优化。真实视频播放/跨平台资源优化对照尚未验收。
7. 最终统一源码门禁、最终release逐平台体积、文档/API表收口、完整三平台kit及远端CI仍需完成。GitHub CLI未登录；正式签名/公证及npm发布未执行。当前可用单host真实测试kit在`/tmp/niva-resource-layout-v4/single-host-test-kit`，不能冒称完整发布kit。

继续时先读本节和最新Git diff，不清工作树；三个Luna Deep子任务分别交付runtime、Native/relay、types/Devtools，现均已要求停止。内存中的旧状态或旧测试总数不能代替本节与对应报告。

2026-09-25。用户最新指令：IPC fallback所需Rust接口由执行方决定，保证基本文件读写、HTTP/HTTPS、exec等高层能力；开始规划并落实全部有效review决定。该指令明确开启实现，覆盖先前“仅review不改代码”的阶段限制。

基线：`47be7b8f39766cad67126e81dcafa38f3e78ddde`，工作分支`codex/architecture-implementation`。原有`docs/architecture-review.md`保留为决策历史，不修改其他session的工作。不自动提交/发布版本，不标v1.0完成；实际平台/发布证据分别记录。

## 当前实施取舍

- 侧边已确认决定也纳入：crypto.timingSafeEqual为纯JS同步模拟，完整遍历、长度校验、调用警告，不宣称时序安全；移除专用Native RPC，保留Rust token鉴权用subtle。
- Niva API默认存在；`injectCommonJs`、`injectEsm`独立、默认false。字段无旧nodeCompat兼容别名。API自身分模块，只有统一bootstrap写Node全局。
- CommonJS内置接口直取Niva；JS/JSON文件通过同步XHR请求Native加载后在页面执行；不支持.node，不自建ESM引擎。ESM为.mjs facade+浏览器importmap，尊重用户覆盖。
- IPC受原生授权约束：普通异步调用使用有界JSON；可信本地页面还可通过有界IPC Channel执行流式/二进制操作，二进制frame在边界Base64编码。已建立的WS只优化大吞吐和流操作，不构成API类别；远端grant仍限unary JSON。同步XHR仅供Node同步兼容API，`process.chdir`改为异步。
- EOF仅终止process.stdin，不自动结束桌面应用；Bridge断线按会话清资源，旧请求不重放。显式独立调试进程必须有明确所有权移交，不以模糊detached状态绕过清理。
- `--resource`、`--config`为正式参数；debug入口明确区分。D07自定义协议调试代理仍是候选，优先确保已有直连调试+明确IPC降级可用，不把未经定案的代理方案宣称已实现。
- 完整UUID标识数据目录；框架日志独立文件，不污染stdio；移除api.host/--stdio/内置NDJSON。
- GUI/CLI共用niva-packager；Windows嵌入单EXE或外置资源绿色ZIP，macOS app/ZIP。无MSI/WiX。保留可选正式签名入口，但无证书不冒称验证过。
- Windows 10/11启动前自检WebView2，使用微软官方在线bootstrapper与签名校验/正常UAC。开发阶段只测试可注入逻辑，不自动在用户机器安装或提权。

## 阶段与文件所有权

| 阶段 | 包/责任人 | 范围 | 状态 |
|---|---|---|---|
| S1-A | Luna Deep startup_boundaries | Native API调度/窗口关闭/main_exec，AR005/011–014 | 代码已交接，待集成测试 |
| S1-B | Luna Deep architecture_inventory | 正式启动参数/UUID/IdCounter/process stdio/日志/Windows引导，AR004/015 | 已交接，待最终平台复验 |
| S1-C | Luna Deep api_js_inventory | 统一packager、外置资源/Range/HEAD、许可与kit体积、可选签名 | 已交接，待最终集成复验 |
| S2-A | Luna Deep startup_boundaries | @niva/runtime TS统一、资源对象、CJS/ESM两个开关与同源类型 | 进行中 |
| S2-B | Luna Deep architecture_inventory | Native模块加载器与IPC文件/HTTP/exec有界接口、断线授权生命周期 | 进行中 |
| S2-C | Luna Deep api_js_inventory | Devtools统一API迁移/单打包入口/保存与CLI错误契约，AR018/019 | 进行中 |
| S3 | 主线程集成+Luna验证 | 全部源码门禁、协议/GUI/平台、完整release体积、文档/CI | 进行中，真实WebView暴露的问题逐项修复 |
| S4 | 真实应用验收包 | 固定Cypress RWA与参考Node对照；JS tsc补测 | 进行中，参考基线通过，Niva兼容仍在修复 |
| S5 | 最后资源优化 OPT-01 | 冻结同功能基线，比较代码整体压缩+媒体按需存储的混合布局、ZIP与现有布局 | 尚未开始，按review明确顺序放在功能正确性之后；external ZIP不等于完成此优化 |

一个文件同一时间一个writer。根Cargo/package manifests与lock由主线程协调，新增依赖先审查必要性和体积；worker不得越界修其他包、再委派或绕过门禁。复用worker已完成检查，仅集成变化/缺口时重跑。旧路径按有效决定移除，不做长期兼容层。

## 跨包接口约定

- 外置资源标记：Windows RT_RCDATA `RESOURCE_MODE`为`external`，资源在exe旁`resources/`；macOS `Contents/Resources/RESOURCE_MODE`为`external`，资源在`Contents/Resources/app/`。嵌入布局保留当前索引+数据格式。由packager与ResourceManager生产/消费一致定义，协议根边界不变。
- Node环境字段由S2 runtime包统一落地；当前原NodeCompat到新包迁移须同时更新Rust build.rs、运行资源索引、Devtools、tests/scripts、类型与CI。
- 平台IPC/`evaluate_script`稳定bridge、可选WS优化bridge和同步XHR的Rust鉴权边界各自保持明确；WS是否可用不改变来源授权。禁止跨bridge重放已提交的副作用；旧活跃请求、句柄和延迟UI任务不能跨会话复活。
- 框架logger初始启动/失败也不得回退stdout/stderr；应用process标准流原样传输，独立日志有大小/轮转限制并隐藏秘密。

### 当前落地接口约定

- 低层传输统一为 `Niva.bridge.call/callSync/stream/streamSend`，释放 `Niva.stream` 给Node stream模块，避免同名冲突；不保留旧 `Niva.api` 或root传输方法别名。
- 嵌入资源的用户CommonJS加载采用真实文件语义：首次需要时将嵌入资源树逐文件解包到本次进程独占临时目录，原子就绪后交给普通文件解析器；目录/绿色模式直接使用资源根，不复制。这样__dirname、相邻资源和readdir保持一致，不另外维护fs虚拟覆盖层。单文件+CJS会产生磁盘解包成本；业务持久数据使用UUID data目录。正常退出清理临时树，异常终止恢复策略须验证。
- 同步loader RPC：`module.resolve(specifier,parentFilename)`返回filename；`module.load`返回`{filename,dirname,type:"commonjs"|"json",source}`。parentFilename为空表示应用资源根，模块内部则传真正的绝对文件路径；支持标准祖先node_modules解析，不能把HTTP资源沙箱误套到已授权的本机Node文件读取。单模块源码最多16MiB。JS维护缓存/循环；Rust仅解析并读取可信调用请求指定的模块。
- IPC文本RPC：`fs.readText(path,encoding?,options?)`、`fs.writeText/appendText(path,text,encoding?,options?)`；首版明确UTF-8。`fs.node`仅允许Rust枚举的stat/lstat/readdir/access/realpath/mkdir/rename/copyFile/rm/unlink/cp JSON元数据/一次性操作。
- `http.requestText(options)`返回`{statusCode,statusMessage,headers,body}`；`process.execText(command,options?)`与`execFileText(file,args?,options?)`返回`{stdout,stderr,status,signal?}`。这是单次文本接口；Node流式对象由WS优化bridge承载，WS未就绪时由IPC Channel承载同一流操作。
- 当前边界：IPC请求JSON最多256KiB；HTTP文本body最多1MiB；IPC响应JSON上限8MiB覆盖文本JSON转义膨胀；exec stdout+stderr合计最多64KiB；高层网络/exec默认10秒、最多30秒，调用选项只可收紧。Native必须在读取阶段限制，不能完整缓冲后才检查。
- 模块解析采用固定版本oxc_resolver；`requestText`与Node `http.request/get`均复用Rust ureq和Niva平台TLS connector，使用`native-tls-no-default`且Cargo锁定图不含WebPKI根证书bundle。Node流式客户端通过WS分块上传/下载和背压确认，由JS只适配Node对象；`createServer`仍由JS处理服务端HTTP协议。

## 完成标准

1. 代码与配置体现全部已确认方向；遗留候选/撤销方案不混入交付，所有AR条目有实现/验证/明确结论。
2. `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets`、`cargo test --workspace`；受影响Windows代码执行target check，记录真机与交叉检查区别。
3. runtime/types TS与构建、Devtools `npm run build --workspace=packages/devtools`、有意义的JS/协议/loader/资源测试通过。pre-commit修成真实失败阻断，不能替代直接检查。
4. 实际macOS原生/WebView验证；Windows可达则真机验证，否则明确未覆盖，不冒称已验收。Windows在线安装器仅在专门测试环境/明确安装操作授权下运行，不能在开发机器静默触发。
5. 最终runtime包含Native、统一JS、索引/loader，逐平台实测严格<3,300,000 bytes；packager/kit/应用资源大小单独统计，hash绑定产物。
6. GUI/CLI输出同一核心；绿色大资源读取、Range/HEAD、签名状态、stdout纯净、EOF/断线清理有可复现证据。
7. 固定Cypress RWA后端CommonJS在Niva内执行，测试工具可用外部Node但不得代跑后端或缺失模块；业务流程、数据持久化与重启清理对照参考Node。失败真实记录，不能改业务源码或测试来掩盖缺失能力。
8. types包生成/打包可用，发布/npm/CI远程运行如缺凭据或未授权另列边界，不能用本地结果冒充对外发布。

## 证据记录

每包完成后追加：变更文件、执行命令/结果、测试范围、产物/hash、平台与未覆盖项。当前S1与S2-A并行推进，后续工作不能标完成。

### 验收准备进度

- Cypress RWA上游固定提交：`79aa5b126fdd951aab2263c8201b52aeb5f2a43c`，源码在隔离目录`/tmp/niva-realworld-79aa5b1`；原业务源码未改。按原yarn.lock准备依赖，暂禁安装生命周期脚本以避免拉取与被测后端无关的Cypress二进制/运行项目钩子，必要官方构建/patch步骤随后显式核对执行；不把这一步算项目通过。
- Windows SSH别名当前连接拒绝，已通知用户恢复；不影响本机实现，Windows真机暂未覆盖。
- GitHub CLI当前未登录，尚无远端CI执行证据；不会将本地检查冒称远端工作流通过。

### 真实项目基线准备（非Niva通过证据）

- 原版Cypress RWA依赖按yarn.lock准备完成；将原TS源码逐文件编译成CommonJS，保留require依赖，不打包/替换业务实现。可复现入口为examples/realworld-acceptance/prepare.mjs，记录源码与锁文件hash。
- 在独立临时目录用参考Node v24.10.0启动原后端，根路由得到HTTP 200及原应用响应；随后主动结束参考进程。这只验证参考环境可启动，未当作Niva验收、业务全流程或UI通过。
- 后续仍需用Niva自身loader执行相同产物，进行登录/会话/持久化等对照。原项目已跟踪业务文件未修改；官方mock AWS配置只作为其本地认证启动配置，未测试第三方OAuth。

- 参考Node业务基线现已通过18项外部HTTP/持久化检查，包含注册、错误密码、登录、会话、银行账户、联系人、模拟付款、磁盘写入、真实logout及后端重启后数据恢复。执行入口`reference.py`，证据`/tmp/niva-realworld-reference-evidence/result.json`；这是参考Node基线，尚不是Niva兼容通过证据。

### 集成与历史材料

- AR-017：相关历史覆盖、官方用例、预算与实现文档已加快照说明；保留原始JSON报告，不将旧58文件/179入口等数字计入新runtime验收。最终新报告仍待运行。
- pre-commit已修复类型检查失败仍继续的问题；`sh -n`通过，隔离模拟npm失败返回7后验证hook退出1且未执行registry检查。直接runtime/types/Devtools检查仍独立执行。

### S1阶段检查快照

- Luna Deep architecture_inventory在runtime目录迁移前执行`cargo check --locked -p niva`与`cargo test --locked -p niva --bin niva`，报告130/130通过；随后新runtime迁移导致旧build.rs输入暂缺，不能把此前结果外推到最终集成。Windows target check发现资源层使用不稳定windows_by_handle，已交资源owner修复；尚无Windows真机。
- Luna Deep api_js_inventory报告packager/win_packager的20+2+10项测试通过及子集cargo check通过，仍在收尾Range/HEAD和许可/脚本。
- 主线程移除HTTP使用的旧public-only socket分支，统一到已鉴权的TCP/TLS接口；保留TLS验证与句柄所属连接隔离。运行验证等待新runtime集成。框架API manager/IPC/快捷键诊断转为独立logger，测试程序的诊断输出不受影响。

### 新验收入口（尚待Niva集成运行）

- `examples/ipc-fallback-smoke/run.py`：真实远端WebView、CSP限制、精确origin授权；另可`--trusted-debug`测试明确调试origin的token fallback。包括文本文件/目录工作流、HTTP/HTTPS/POST/404、exec、同步与二进制绕过拒绝，以及active心跳与页面阻塞后Native终止子进程。Native仍存活时检查延迟副作用，避免用App退出掩盖资源泄漏。
- `examples/realworld-acceptance/niva.py`：仅以Niva自身require加载原始转译后端，Python只驱动HTTP；没有host Node替代服务。待运行。
- `examples/stdio-host/verify.py`：普通管道、UTF-8拆块、EOF只结束输入、stdout/stderr纯净检查；示例配置显式injectCommonJs，等待生成运行时后实测。
- 固定Node v22.14.0测试host已安装在隔离`/tmp/niva-validation-node22`，不修改全局PATH；仅供官方契约runner，不作为被测Niva实现。

- Devtools迁移已由Luna Deep api_js_inventory通过一轮直接TS+Vite build；Vite dev server实测导入解析到自有virtual facade，Node vm验证facade引用Niva同一对象。它不依赖debug URL的HTML importmap，也不是实际Niva WebView通过证据。网站18个相关API/配置/事件页面已同步新接口与边界，Docusaurus build通过，尚未发布。

### 第一轮真实WebView发现（未计通过）

- Native `cargo check --locked --workspace`与162项完整Native单测已通过，包含HTTP/注册层/loader/取消；runtime118项JS测试通过的快照仍需真机验证。
- 冻结debug `8fa9fafae149…` 的stdio启动无输出；真实页面诊断确认`niva://app` origin与Web Crypto可用，但Niva尚未创建，runtime停在OS factory。根因是OS factory在bootstrap建立Niva之前提前取快照；旧VM预造Niva掩盖了它。已要求/实现移除预造Niva并修初始化顺序，不用VM通过代替真实页面。
- 更新debug `8028a0c4b24f…`后Native日志暴露call id被JS序列化成string而Rust期望u64；另发现初次WS连接中被过早当IPC而影响stdio。两项由runtime修复，接下来重跑真实管道与fallback验收。原失败报告保留在`/tmp/niva-architecture-stdio-evidence*`，不计通过。
- 原版RWA前端依照官方mock配置和官方react-virtualized patch准备后，`yarn build`（含原项目tsc）通过；仍不等于Niva UI/后端兼容通过。

### 第二轮真实WebView与跨平台编译

- 冻结debug SHA256 `4209f1203d0344961b91c44a21d6d8063e088d82ff20e52f45118e6df6180709`：stdio五项通过，包括首条stdout由应用产生、UTF-8拆块、EOF事件、EOF后窗口存活、框架日志不混入stderr。`/tmp/niva-architecture-stdio-evidence-v3/result.json`。
- 同一产物的`process.exit`真实子进程清理通过：子进程确认已启动后退出Niva，等待其预定写文件时间仍无孤儿副作用。`/tmp/niva-exit-cleanup-v3/result.json`。
- 四开关矩阵的基础/CJS模式通过，包含严格CSP下同步XHR加载真实JS和缓存身份。两个ESM模式失败：发现JS和Native重复注入map，JS还错误生成`node:module` facade路径；正在修复，不能以资源HTTP 200代替模块执行通过。
- 远端授权IPC真实调用发现Rust字段`session_id`与JS `sessionId`不一致；二进制/同步拒绝项成功，但业务能力未通过。Native owner负责统一协议并补真实JS形状测试。
- RWA在真实WebView中加载到depd依赖时遇到非V8 CallSite接口缺失，保留失败报告`/tmp/niva-realworld-evidence-v3/result.json`，不修改业务代码规避。
- Windows交叉`cargo check --locked --target x86_64-pc-windows-msvc -p niva`通过，修复了Job Object分支Ok遮蔽与frame事件借用生命周期错误；不是Windows真机通过证据。
- 新增`examples/typescript-cli-smoke/run.py`：未修改的TypeScript CLI在Niva内执行，分别核对多文件/标准库编译输出与类型错误退出/不产物；不使用host Node执行编译器。

### 迁移后的独立验收用例

- 同一4209快照的三个headless Native组（12/4/14条，含各组重复的生命周期检查）与7项bridge/event方法通过。原四个自定义module registry入口已经删除，模块身份由单独CommonJS/ESM测试覆盖，不能继续硬编码“11项bridge通过”。
- Node macOS smoke迁移到真实Node契约后，174项方法/别名、16组全部通过：callback fs/zlib经promisify使用；HTTP通过独立Python服务验证真实response stream、POST字节与pipe；promise pipeline按Node返回undefined。旧HTTP post/result/response/body便捷层、os.sep/os.delimiter不属于现行接口，明确退役而非伪称通过。querystring自定义decoder的空格输入以参考Node核对为%20。
- 完整窗口组目前不能验收：显示器休眠时monitor.list为空；短暂唤醒后能通过monitor与几何/可见性检查，但焦点读回尚失败。测试保留严格实际效果，未用源码调用成功替代原生焦点结果。
- 系统只读状态进一步确认`CGSSessionScreenIsLocked=Yes`；已请求用户解锁以完成桌面效果验收，不尝试绕过锁屏。
- website生产构建与Devtools七项契约检查通过。CI移除已经迁入runtime的旧脚本路径，并加入真实bootstrap/stdio/退出清理/tsc驱动；尚未触发远端CI。

### 第一份完整release尺寸与IPC运行证据

- `cargo build --locked --release -p niva -p niva-packager`通过；macOS ARM64 runtime实际3,073,008 bytes，严格小于3,300,000。冻结路径`/tmp/niva-architecture-release-23082c971a8f`，SHA256 `23082c971a8f9884e891cf1ee373334887a072b5069222009cd14c4ce655a4f5`。包含新增Native HTTP/exec/loader/fd代码、内嵌bootstrap/ESM及索引；资源优化前快照，不是最终产物尺寸。
- 此次runtime源码typecheck已通过，资产构建标记`typesChecked:false`，隔离types生成仍在完成；不将该release构建等同于types最终门禁通过。
- 真正只有IPC的远端页面中，文本文件读写/追加、stat、目录工作流、HTTP 200/404/POST、正常TLS的HTTPS、execFile、拒绝同步与二进制、4秒任务心跳、页面阻塞后lease失效与Native杀子进程均通过。execText仍失败：错误地把完整shell命令当文件名；正在修正，整个套件保持失败状态。报告`/tmp/niva-ipc-fallback-v4/result.json`。
- 同轮Native新增WS会话约束和JS尚未同步的call字段导致矩阵WS请求报session错误，下一个统一快照必须完整复测；不沿用前轮WS通过当最终证据。RWA已越过depd，但尚缺tty接口，仍未启动完成。

### 完整类型、真实Bridge与打包进展

- 后续定位WS session失败的实际集成根因：HTTP/WebSocket pump消费Hello后未把session交给ApiManager；已修复同一生产握手路径，非法hello拒绝并关闭。之前“只是JS字段未同步”的猜测被此实证取代。
- 冻结release `e269ac719067320e2a3f152b56011e42a8082dd28806d4416a26b3569c0bc3b7`，macOS ARM64 3,106,128 bytes；从二进制定位内嵌压缩blob并解压比对，确认与fingerprint `9e54a9ad…`的bootstrap字节完全一致。
- 此产物真实四开关+CSP+用户importmap覆盖共5组通过；TypeScript CLI成功编译/声明/source map/exit0与错误诊断/不产物/exit1两组通过。报告`/tmp/niva-bootstrap-matrix-v6/result.json`、`/tmp/niva-typescript-cli-v6/result.json`。
- IPC远端精确授权24项全部通过，含exec shell修复、写入/响应/输出超限、超时、禁用fd/窗口创建、心跳和失联清理。`/tmp/niva-ipc-fallback-v5/result.json`绑定上一份完整产物`1a797193…`；明确debug-token模式尚有未回报问题，继续调查，不能外推为所有调试模式通过。
- embedded/external真实macOS `.app` ZIP均通过ad-hoc验签、33MiB以上资源HEAD/Range/416/413、许可证、包内CommonJS相邻文件读取、ESM身份7项检查，`/tmp/niva-resource-layout-v5/result.json`。样本是周期字节，不能用其压缩率当媒体收益。
- Node22.14官方固定子集JS48/48、Native10/10通过；Native报告`/tmp/niva-upstream-native-report-e269ac719067-r2.json`。Node runtime单测快照128/128；后来版本元数据调整后的130/130与JS48/48也通过。它们不是完整Node测试集通过声明。
- Rust该快照fmt/check/clippy/workspace tests与Windows target check通过：Niva175、packager22、validation2、win_packager10。Clippy有warnings，Windows无真机证据。
- 正式types typecheck/四种独立tarball消费者与Devtools tsc+Vite均通过。默认browser不自动安装Node globals；Node工具入口保留标准Node精确签名，CommonJS显式启用。两者不混入同一个全局类型图。
- RWA下一缺口是依赖读取process.versions.node。实现决策改为公开Node兼容目标`process.version=v22.14.0`、`versions.node/nodeCompat=22.14.0`，真实产品版本保留`versions.niva`；不表示内置Node/V8，也不利用此字段删减验收分母。
- DevtoolsCLI三项已实际完成（缺配置/缺kit的结构化stderr+exit1、真实统一packager成功+exit0）；但测试发现随机UUID仍共享macOS WebKit localStorage。该隔离问题另行修复，不能以测试清理kit键代替产品隔离。原默认store不删除，不迁移用户数据。

## 用户恢复工作 — Native复用审查与未完事项继续

2026-09-25。用户要求完成上一轮额度暂停前的未完事项，并明确新增规则：Rust已具备的能力由Rust作为实现来源，JS Node API优先做适配；缺流、背压、取消等语义时扩展Native桥，不降低JS公开契约。

- release基线仍为macOS ARM64 `target/release/niva` 3,106,128 bytes；降至严格小于3,000,000至少需省106,129 bytes。上一版实施报告为2,772,208 bytes。旧NodeCompat资源索引合计405,669 compressed bytes；当前统一runtime资源索引合计190,465 compressed bytes。因此整体增量并非由新JS bundle单独造成；其它Native代码/依赖需按真实release拆分。
- ureq feature审计发现`native-tls`会拉入`webpki-root-certs`，但Niva只用`RootCerts::PlatformVerifier`。直接启用`native-tls-no-default`会隐藏ureq connector，所以commit `8fc43f7`改为Niva本地TLS Connector包装已有`native-tls`；保留系统根、SNI、host验证、超时/取消，排除Mozilla根证书bundle。
- `oxc_resolver`无已启用可选feature；`yarn_pnp`关闭。little-endian目标的simd-json是oxc_resolver无条件依赖并用于package.json解析，不能误记为可关的feature。
- `runtime/http.ts`的`requestText`已调用Rust `http.requestText`；Node `http.request/get`仍在JS通过Rust net/TLS socket自行做HTTP framing/parser；`createServer`是另一项服务端能力。review结论要求把高层HTTP/HTTPS客户端契约迁到Rust ureq的流式桥接后由JS适配，普通IPC页面保留有界`requestText`。
- 本文首部的“quota暂停”记录仅为历史。按用户恢复指令，继续逐项完成真实RWA、debug-entry/token IPC、macOS UUID WebKit存储隔离、OPT-01及源码/平台/完整release验收；硬件、登录或目标机不可用的项目保留证据并如实标未验收。

### 当前提交基线与体积验证 — 2026-09-25

- 按用户要求先提交当前可运行实现，再调查尺寸：commit `8fc43f7401758d5193a5df3ccc17b1980eedcb26`（`Implement unified Niva runtime and native APIs`）；commit前工作树干净，pre-commit runtime/types检查通过。
- ureq改用`native-tls-no-default`，由Niva本地Connector包装现有`native-tls`；仅允许`RootCerts::PlatformVerifier`，保留平台根、SNI、hostname验证、handshake超时/取消。`Cargo.lock`不再含`webpki-root-certs`。Rust工作区测试中该TLS路径通过本地不可信证书拒绝、handshake取消及超时；联网系统证书单测标记ignored。
- 同平台完整release重建：macOS ARM64 **2,940,928 bytes**，SHA256 `05e49beeacc18d29329b8a9c51be385962b8dbff2cce87afc616c2a9c3bed4ae`，冻结于`/tmp/niva-native-reuse-release-05e49beeacc1`。比提交前同平台3,106,128 bytes少165,200 bytes；低于3,000,000目标59,072 bytes。只证实macOS ARM64，Windows/Mac Intel待目标构建。
- 上述冻结release真实WebView IPC 24项全过，报告`/tmp/niva-ipc-fallback-postcommit/result.json`，SHA与产物一致；包括HTTPS通过系统证书校验、文件/目录、exec超限/超时及失联子进程清理。
- commit验证：`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets`通过（Clippy有现存warnings）；`cargo test --workspace`中Niva 178 passed/1 ignored、niva-packager 22/22、validation 2/2、win-packager 10/10；`cargo check --target x86_64-pc-windows-msvc -p niva`通过但不代表Windows真机。
- TypeScript runtime `npm test --workspace=packages/runtime`通过131/131；types typecheck、Devtools TS+Vite build及commit hook检查通过。真实RWA在`node:constants`处的旧报告为0/18；修复已进入commit，已启动新一轮验收但结果待报。trusted-debug IPC普通请求前22项过，末尾失联lease用例promise未回报；等待定位。macOS UUID storage隔离仍未接入，macOS<14策略正在等产品选择。
- 用户随后取消更换项目的工作：D19继续保留原Cypress Real World App为主验收对象；`require(ESM)`不实现。RealWorld依赖`dinero.js` ESM-only导致的CJS启动失败作为已确认边界报告，不改上游应用，也不为它改变CJS-only架构；暂停新候选项目的准备和安装。

### 2026-09-26 收尾实现、平台证据与最终尺寸

- 按Rust-first规则把Node `http.request/get`客户端改为受信WS上的`http.requestStream`；Rust ureq负责HTTP/HTTPS framing、TLS、请求/响应流、取消和逐块背压，JS只适配`ClientRequest/IncomingMessage`。有界IPC `http.requestText`继续复用同一Rust客户端；`createServer`职责不同，仍由JS协议层复用Native TCP/TLS。缺少Agent连接池、真实底层`net.Socket`、自定义连接、upgrade、响应trailer、自定义reason phrase；Native总超时与Node idle timeout不完全一致。
- `fs.cp`无filter时由Rust一次递归复制；有filter时JS执行Node异步filter和遍历，Native完成目录/文件/符号链接复制，内容不经JS或IPC。按Node v22.14补齐常用force/errorOnExist、递归、自指/循环、软链接、时间戳及权限边界。`COPYFILE_FICLONE`回退普通复制，`COPYFILE_FICLONE_FORCE`明确ENOTSUP。
- 本地协议改成canonical app UUID派生`niva-<32hex>`。macOS/Linux origin为`niva-<uuid>://app`，Windows/Android按Wry映射为`http://niva-<uuid>.app`；`niva://app`只保留为配置别名。资源、页面origin、WS/CORS/IPC来源均按当前app精确校验。Linux共享WebContext每app只注册一次scheme。
- `ureq`只启用`native-tls-no-default`并使用Niva复用的系统TLS connector；ureq feature图不再把121张未使用的Mozilla根证书（129,143原始字节）链接进Niva。`webpki-roots`仍在workspace lockfile，由`niva-packager`的`reqwest`使用；不能把它误当作Niva `ureq`路径。`oxc_resolver`没有误开的可选feature；`yarn_pnp`关闭，simd-json是resolver无条件依赖。未换验收项目、未为RWA的ESM-only `dinero.js`实现`require(ESM)`，不改原CJS-only选择。
- macOS 26.6.2固定bundle identity的最终release `.app` A/A/B本地存储smoke通过：最终release SHA `e0ba2d28a1d9c9d8977a7a72833908def98d66606bb72d3e3f3ac672edfafc0b`；同UUID重启读回`persisted`，异UUID读到`null`。记录`/tmp/niva-per-app-origin-smoke/release-result-final.json`。build脚本最低目标macOS 11.0，但未在macOS 11真机验证。第二原生窗口可创建且与主窗口取得同scheme，但没有捕获子页面marker；多窗口页面加载/本地存储未验收。
- 最终完整macOS ARM64 release：`cargo build --locked --release -p niva`通过，`target/release/niva` **2,974,288 bytes**，SHA与`/tmp/niva-final-release-e0ba2d28a1d9`相同，低于用户3,000,000目标 **25,712 bytes**。较3,106,128字节的最初基线少131,840字节；较去掉WebPKI证书bundle后的2,940,928字节基线新增33,360字节，增量来自后续HTTP stream、fs.cp和UUID协议功能。
- 最终门禁：`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets`、`cargo test --workspace`通过（Niva 189 passed/2 ignored、niva_packager 22、validation 2、win_packager 10）；runtime全量测试136/136、types typecheck及Devtools `tsc+Vite`通过。新增真实macOS release WebView NodeCompat smoke通过178项，覆盖HTTP GET、POST上传和响应流。Windows MSVC target check使用共享缓存通过，但未做Windows真机验证。Devtools Vite保留现有大chunk warning。
- 最终release真实WebView IPC矩阵在Mac解锁且Raise测试窗口后，以SHA `159b98...`的前一release `.app` 分别用显式remote grants和trusted-debug token跑过24/24；报告`/tmp/niva-ipc-visible-foreground/result.json`和`/tmp/niva-ipc-visible-foreground-trusted-debug/result.json`。随后最终release SHA `e0ba...`在Mac锁定、`document.hidden=true`环境下的完整remote矩阵停在最后lease reply，前22项通过；该最终SHA的独立lease-only通过。此差异只改了Node HTTP stream请求头pair编码；当前lease回复的最终完整remote E2E仍需在解锁的主机复跑。子进程已被Native清理，未产生延迟副作用。
- `ureq`只启用`native-tls-no-default`并使用Niva复用的系统TLS connector；ureq feature图不再把121张未使用的Mozilla根证书（129,143原始字节）链接进Niva。`webpki-roots`仍在workspace lockfile，由`niva-packager`的`reqwest`使用；不能把它误当作Niva `ureq`路径。`oxc_resolver`没有误开的可选feature；`yarn_pnp`关闭，simd-json是resolver无条件依赖。未换验收项目、未为RWA的ESM-only `dinero.js`实现`require(ESM)`，不改原CJS-only选择。
