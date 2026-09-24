# Niva API 测试覆盖矩阵

本表是覆盖跟踪清单，不代表整体覆盖验收。公开 API 清单以 `packages/types/Niva_zh.d.ts` 中 `NivaObj.api` 引用的命名空间接口里声明的 `MethodSignature` 为准，共 167 个方法；不计属性，也不将类型到 handler 的名字映射视为行为覆盖。初始化脚本中的 300 次通用代理调用压力测试同样不算逐方法行为测试。直接行为测试与 helper 级测试分别标注；helper 级用例验证底层 helper，不视为完整 Niva.api handler 覆盖。原生观察仅代表所列 smoke 的实际操作。2026-09-24 的 macOS 顺序套件 `examples/macos-api-smoke/run_all.py` 已实际调用 167/167 个 `Niva.api` 方法：161 个有行为断言或外部原生观察，6 个仅证实调用成功，效果仍待独立验证；另有 9/9 个 `NivaObj` bridge 方法在真实 WebView 中通过。NodeCompat 的 185 项真机 WebView 检查另见 `docs/node-compat-test-matrix.md`。

## Niva.api（167 个方法）

| Method | unit/JS behavior test pointer | macOS native observed | Windows native observed | remaining case needed |
|---|---|---|---|---|
| `Niva.api.clipboard.read` | 未发现逐方法行为断言 | macOS 真机 PASS（clipboard-shortcut，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.clipboard.write` | 未发现逐方法行为断言 | macOS 真机 PASS（clipboard-shortcut，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.dialog.showMessage` | 未发现逐方法行为断言 | macOS 真机 PASS（dialogs，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.dialog.pickFile` | 未发现逐方法行为断言 | macOS 真机 PASS（dialogs，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.dialog.pickFiles` | 未发现逐方法行为断言 | macOS 真机 PASS（dialogs，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.dialog.pickDir` | 未发现逐方法行为断言 | macOS 真机 PASS（dialogs，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.dialog.pickDirs` | 未发现逐方法行为断言 | macOS 真机 PASS（dialogs，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.dialog.saveFile` | 未发现逐方法行为断言 | macOS 真机 PASS（dialogs，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.extra.hideApplication` | 未发现逐方法行为断言 | macOS 真机 PASS（system-supervised，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.extra.showApplication` | 未发现逐方法行为断言 | macOS 真机 PASS（system-supervised，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.extra.hideOtherApplications` | 未发现逐方法行为断言 | macOS 真机 PASS（system-supervised，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.extra.setActivationPolicy` | 未发现逐方法行为断言 | macOS 真机 PASS（system-supervised，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.extra.getActiveWindowId` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.extra.focusByWindowId` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.fs.stat` | crates/niva/src/app/api/fs.rs — `stat_and_exists_cover_files_directories_and_missing_paths` (helper, not full handler) | macOS 真机 PASS（headless，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.fs.exists` | crates/niva/src/app/api/fs.rs — `stat_and_exists_cover_files_directories_and_missing_paths` (helper, not full handler) | macOS 真机 PASS（headless，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.fs.read` | `bridge-api-overrides.test.mjs` — `fs.read returns streamed UTF-8 and base64 bytes` | macOS 真机 PASS（headless，逐方法） | Windows 真机 [run_stream.py](../examples/windows-smoke/run_stream.py) 150 KB 流与输出断言通过 | 其他平台或选项分支待测 |
| `Niva.api.fs.write` | `bridge-api-overrides.test.mjs` — `fs.write sends content through writeStream` | macOS 真机 PASS（headless，逐方法） | Windows 真机 [run_stream.py](../examples/windows-smoke/run_stream.py) 150 KB 流与输出断言通过 | 其他平台或选项分支待测 |
| `Niva.api.fs.append` | `bridge-api-overrides.test.mjs` — `fs.append sends content through writeStream` | macOS 真机 PASS（headless，逐方法） | Windows 真机 [run_stream.py](../examples/windows-smoke/run_stream.py) 150 KB 流与输出断言通过 | 其他平台或选项分支待测 |
| `Niva.api.fs.move` | crates/niva/src/app/api/fs.rs — `move_dispatches_files_and_directories_and_removes_sources` (helper, not full handler) | macOS 真机 PASS（headless，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.fs.copy` | crates/niva/src/app/api/fs.rs — `copy_options_apply_camel_case_fields_and_defaults`; `copy_dispatches_files_and_directories_and_applies_skip_exist` (helper, not full handler) | macOS 真机 PASS（headless，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.fs.remove` | crates/niva/src/app/api/fs.rs — `remove_deletes_a_file_or_a_directory_tree` (helper, not full handler) | macOS 真机 PASS（headless，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.fs.createDir` | crates/niva/src/app/api/fs.rs — `create_dir_is_single_level_and_create_dir_all_builds_parents` (helper, not full handler) | macOS 真机 PASS（headless，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.fs.createDirAll` | crates/niva/src/app/api/fs.rs — `create_dir_is_single_level_and_create_dir_all_builds_parents` (helper, not full handler) | macOS 真机 PASS（headless，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.fs.readDir` | crates/niva/src/app/api/fs.rs — `read_dir_lists_only_immediate_entry_names` (helper, not full handler) | macOS 真机 PASS（headless，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.fs.readDirAll` | crates/niva/src/app/api/fs.rs — `read_dir_all_recurses_and_excludes_matching_paths` (helper, not full handler) | macOS 真机 PASS（headless，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.host.send` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.monitor.list` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.monitor.current` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.monitor.primary` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.monitor.fromPoint` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.os.info` | crates/niva/src/app/api/os.rs — `info_includes_os_architecture_and_version_as_strings` (helper, not full handler) | macOS 真机 PASS（headless，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.os.dirs` | crates/niva/src/app/api/os.rs — `dirs_always_include_app_paths_and_add_user_directories_when_available` (helper, not full handler) | macOS 真机 PASS（headless，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.os.sep` | crates/niva/src/app/api/os.rs — `separator_and_line_ending_match_the_target_platform` (helper, not full handler) | macOS 真机 PASS（headless，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.os.eol` | crates/niva/src/app/api/os.rs — `separator_and_line_ending_match_the_target_platform` (helper, not full handler) | macOS 真机 PASS（headless，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.os.locale` | crates/niva/src/app/api/os.rs — `locale_uses_system_value_or_en_us_fallback` (helper, not full handler) | macOS 真机 PASS（headless，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.process.pid` | crates/niva/src/app/api/process.rs — `process_metadata_methods_return_the_current_process_values` (helper, not full handler) | macOS 真机 PASS（headless，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.process.currentDir` | crates/niva/src/app/api/process.rs — `process_metadata_methods_return_the_current_process_values` (helper, not full handler) | macOS 真机 PASS（headless，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.process.currentExe` | crates/niva/src/app/api/process.rs — `process_metadata_methods_return_the_current_process_values` (helper, not full handler) | macOS 真机 PASS（headless，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.process.env` | crates/niva/src/app/api/process.rs — `env_and_args_are_returned_as_json_map_and_ordered_array` (helper, not full handler) | macOS 真机 PASS（extended，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.process.args` | crates/niva/src/app/api/process.rs — `env_and_args_are_returned_as_json_map_and_ordered_array` (helper, not full handler) | macOS 真机 PASS（headless，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.process.setCurrentDir` | crates/niva/src/app/api/process.rs — `set_current_dir_changes_only_an_isolated_child_process` (helper, not full handler) | macOS 真机 PASS（extended，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.process.exit` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.process.exec` | `bridge-api-overrides.test.mjs` — `process.exec collects stdout and stderr into separate strings`; crates/niva/src/app/api/process.rs — `exec_command_applies_arguments_env_directory_and_preserves_streams_and_status`; `exec_command_reads_detached_option_and_defaults_it_to_false` (helper, not full handler) | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_stream.py](../examples/windows-smoke/run_stream.py) 150 KB 流与输出断言通过 | 其他平台或选项分支待测 |
| `Niva.api.process.open` | 未发现逐方法行为断言 | macOS 真机 PASS（system-supervised，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.process.version` | crates/niva/src/app/api/process.rs — `process_metadata_methods_return_the_current_process_values` (helper, not full handler) | macOS 真机 PASS（headless，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.resource.exists` | crates/niva/src/app/api/resource.rs — `exists_checks_a_resource_path_and_rejects_missing_or_escaping_files` (helper, not full handler) | macOS 真机 PASS（headless，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.resource.read` | `bridge-api-overrides.test.mjs` — `resource.read returns streamed resource bytes` | macOS 真机 PASS（headless，逐方法） | Windows 真机 [run_stream.py](../examples/windows-smoke/run_stream.py) 150 KB 流与输出断言通过 | 其他平台或选项分支待测 |
| `Niva.api.resource.extract` | crates/niva/src/app/api/resource.rs — `extract_writes_resource_bytes_and_preserves_destination_on_missing_source` (helper, not full handler) | macOS 真机 PASS（extended，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.shortcut.register` | 未发现逐方法行为断言 | macOS 真机 PASS（clipboard-shortcut，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.shortcut.unregister` | 未发现逐方法行为断言 | macOS 真机 PASS（clipboard-shortcut，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.shortcut.unregisterAll` | 未发现逐方法行为断言 | macOS 真机 PASS（clipboard-shortcut，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.shortcut.list` | `shortcut.rs` — `list_serializes_entries_with_the_public_id_and_accelerator_fields`（返回结构 helper；未走真实快捷键管理器） | macOS 真机 PASS（clipboard-shortcut，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.tray.create` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.tray.destroy` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.tray.destroyAll` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.tray.list` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.tray.update` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.webview.evaluateScript` | `webview.rs` — `evaluate_script_limit_counts_utf8_bytes_and_includes_the_boundary`（输入验证 helper；未运行 Wry） | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.webview.loadUrl` | `webview.rs` — `load_url_accepts_only_absolute_http_or_https_urls`（输入验证 helper；未运行 Wry） | macOS 真机 PASS（default，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.webview.loadHtml` | 未发现逐方法行为断言 | macOS 真机 PASS（system-supervised，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.webview.reload` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.webview.url` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.webview.print` | 未发现逐方法行为断言 | macOS 真机 PASS（system-supervised，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.webview.goBack` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.webview.goForward` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.webview.canGoBack` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.webview.canGoForward` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.webview.cookies` | `webview.rs` — `cookie_arguments_are_parsed_and_formatted_as_set_cookie_values`（输出格式 helper；未读 Wry Cookie store） | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.webview.cookiesForUrl` | `webview.rs` — `cookie_arguments_are_parsed_and_formatted_as_set_cookie_values`（输出格式 helper；未测 URL 筛选） | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.webview.setCookie` | `webview.rs` — `cookie_arguments_are_parsed_and_formatted_as_set_cookie_values`（输入解析 helper；未写 Wry Cookie store） | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.webview.deleteCookie` | `webview.rs` — `cookie_arguments_are_parsed_and_formatted_as_set_cookie_values`（输入解析 helper；未删 Wry Cookie） | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.webview.clearAllBrowsingData` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.webview.isDevtoolsOpen` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Wry 0.57 WebView2 固定返回 false；不能据此验收状态 | 其他平台或选项分支待测 |
| `Niva.api.webview.openDevtools` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Wry 0.57 会请求打开；受监督 UI 结果待记录 | 其他平台或选项分支待测 |
| `Niva.api.webview.closeDevtools` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Wry 0.57 WebView2 为空操作；不能验收关闭 | 其他平台或选项分支待测 |
| `Niva.api.webview.baseUrl` | `webview.rs` — `local_webview_urls_include_the_server_port_and_window_file_token`（URL 构造 helper；未走真实服务） | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.webview.baseFileSystemUrl` | `webview.rs` — `local_webview_urls_include_the_server_port_and_window_file_token`（URL 构造 helper；未走文件服务） | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.current` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.open` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.close` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.list` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.sendMessage` | 未发现逐方法行为断言 | macOS 真机 PASS（window-supervised，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.setMenu` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.hideMenu` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.showMenu` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.isMenuVisible` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.scaleFactor` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.innerPosition` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.outerPosition` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.setOuterPosition` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.innerSize` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.setInnerSize` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.outerSize` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.setMinInnerSize` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.window.setMaxInnerSize` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.window.setWindowIcon` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.setTheme` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.dragResizeWindow` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.window.setProgressBar` | 未发现逐方法行为断言 | macOS 真机 PASS（system-supervised，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.window.requestRedraw` | 未发现逐方法行为断言 | macOS 真机调用成功（window-supervised）；效果未独立观察 | 未观察 | macOS 原生效果及其他平台待验证 |
| `Niva.api.window.setImePosition` | 未发现逐方法行为断言 | macOS 真机调用成功（window-supervised）；效果未独立观察 | 未观察 | macOS 原生效果及其他平台待验证 |
| `Niva.api.window.setBackgroundColor` | 未发现逐方法行为断言 | macOS 真机 PASS（window-supervised，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.window.setFocusable` | 未发现逐方法行为断言 | macOS 真机 PASS（window-supervised，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.window.setTitle` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.title` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.isVisible` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.setVisible` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.isFocused` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.window.setFocus` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.window.isResizable` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.setResizable` | 未发现逐方法行为断言 | macOS 真机 PASS（default，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.isMinimizable` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.setMinimizable` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.isMaximizable` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.setMaximizable` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.isClosable` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.isMinimized` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.setMinimized` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.isMaximized` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.setMaximized` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.setClosable` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.isDecorated` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.setDecorated` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.fullscreen` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.window.setFullscreen` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.window.setAlwaysOnTop` | 未发现逐方法行为断言 | macOS 真机 PASS（window-supervised，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.setAlwaysOnBottom` | 未发现逐方法行为断言 | macOS 真机 PASS（window-supervised，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.window.requestUserAttention` | 未发现逐方法行为断言 | macOS 真机调用成功（window-supervised）；效果未独立观察 | 未观察 | macOS 原生效果及其他平台待验证 |
| `Niva.api.window.setContentProtection` | 未发现逐方法行为断言 | macOS 真机 PASS（window-supervised，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.setVisibleOnAllWorkspaces` | 未发现逐方法行为断言 | macOS 真机调用成功（window-supervised）；效果未独立观察 | 未观察 | macOS 原生效果及其他平台待验证 |
| `Niva.api.window.setCursorIcon` | 未发现逐方法行为断言 | macOS 真机 PASS（window-supervised，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.window.cursorPosition` | 未发现逐方法行为断言 | macOS 真机 PASS（window-supervised，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.window.setCursorPosition` | 未发现逐方法行为断言 | macOS 真机 PASS（window-supervised，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.window.setCursorGrab` | 未发现逐方法行为断言 | macOS 真机调用成功（window-supervised）；效果未独立观察 | 未观察 | macOS 原生效果及其他平台待验证 |
| `Niva.api.window.setCursorVisible` | 未发现逐方法行为断言 | macOS 真机 PASS（window-supervised，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.window.dragWindow` | 未发现逐方法行为断言 | macOS 真机 PASS（drag，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.window.setIgnoreCursorEvents` | 未发现逐方法行为断言 | macOS 真机调用成功（window-supervised）；效果未独立观察 | 未观察 | macOS 原生效果及其他平台待验证 |
| `Niva.api.window.theme` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.window.blockCloseRequested` | 未发现逐方法行为断言 | macOS 真机 PASS（window-supervised，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.windowExtra.setEnable` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.windowExtra.setTaskbarIcon` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.windowExtra.theme` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.windowExtra.resetDeadKeys` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.windowExtra.beginResizeDrag` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.windowExtra.setSkipTaskbar` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.windowExtra.setUndecoratedShadow` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.windowExtra.setOverlayIcon` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.windowExtra.setRtl` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.windowExtra.hasUndecoratedShadow` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | Windows 真机 [run_api.py](../examples/windows-smoke/run_api.py) 逐方法行为断言通过（68 项整套） | 其他平台或选项分支待测 |
| `Niva.api.windowExtra.simpleFullscreen` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.windowExtra.setSimpleFullscreen` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.windowExtra.hasShadow` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.windowExtra.setHasShadow` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.windowExtra.setIsDocumentEdited` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.windowExtra.isDocumentEdited` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.windowExtra.setAllowsAutomaticWindowTabbing` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.windowExtra.allowsAutomaticWindowTabbing` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.windowExtra.setTabbingIdentifier` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.windowExtra.tabbingIdentifier` | 未发现逐方法行为断言 | macOS 真机 PASS（extended，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.windowExtra.setTrafficLightInset` | 未发现逐方法行为断言 | macOS 真机 PASS（window-supervised，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.windowExtra.setActivationPolicyAtRuntime` | 未发现逐方法行为断言 | macOS 真机 PASS（system-supervised，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.windowExtra.setDockVisibility` | 未发现逐方法行为断言 | macOS 真机 PASS（system-supervised，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.api.windowExtra.setBadgeLabel` | 未发现逐方法行为断言 | macOS 真机 PASS（system-supervised，逐方法） | 未观察 | 其他平台或选项分支待测 |

## NivaObj bridge 方法（9 个）

9 个方法指函数成员：`require`、`registerModule`、`import`、事件订阅/移除 4 项、`call`、`stream`、`streamSend`；只读属性 `bridgeVersion` 不计入这 9 项。

| Method | unit/JS behavior test pointer | macOS native observed | Windows native observed | remaining case needed |
|---|---|---|---|---|
| `Niva.require` | `bridge-api-overrides.test.mjs` — `Niva module registration, require, and import preserve module identity` | macOS 真机 PASS（run_bridge_top_level.py，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.registerModule` | `bridge-api-overrides.test.mjs` — `Niva module registration, require, and import preserve module identity` | macOS 真机 PASS（run_bridge_top_level.py，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.import` | `bridge-api-overrides.test.mjs` — `Niva module registration, require, and import preserve module identity` | macOS 真机 PASS（run_bridge_top_level.py，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.addEventListener` | `bridge-api-overrides.test.mjs` — `Niva event subscriptions match exact, namespace, and wildcard names` | macOS 真机 PASS（run_bridge_top_level.py，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.removeEventListener` | `bridge-api-overrides.test.mjs` — `Niva event subscriptions match exact, namespace, and wildcard names` | macOS 真机 PASS（run_bridge_top_level.py，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.removeAllEventListeners` | `bridge-api-overrides.test.mjs` — `Niva event subscriptions match exact, namespace, and wildcard names` | macOS 真机 PASS（run_bridge_top_level.py，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.call` | `bridge-api-overrides.test.mjs` — `Niva.call resolves successful replies and rejects native errors` | macOS 真机 PASS（run_bridge_top_level.py，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.callSync` | API Manager 同步方法门禁与 HTTP framing 测试 | macOS WebView 同步文件/进程/系统调用 PASS | 未观察 | Windows 仅 target check；同步调用不支持 UI/流式方法 |
| `Niva.stream` | `bridge-api-overrides.test.mjs` — `Niva.stream cancellation and streamSend use the call ID and binary frame` | macOS 真机 PASS（run_bridge_top_level.py，逐方法） | 未观察 | 其他平台或选项分支待测 |
| `Niva.streamSend` | `bridge-api-overrides.test.mjs` — `Niva.stream cancellation and streamSend use the call ID and binary frame` | macOS 真机 PASS（run_bridge_top_level.py，逐方法） | 未观察 | 其他平台或选项分支待测 |

## 计数口径

- 清单行数：167 个 `Niva.api` 方法 + 9 个 `NivaObj` bridge 方法；bridgeVersion 属性另行说明，不算方法。
- 已找到逐方法 JS 行为测试：8/167 个 `Niva.api` 方法；另有 24 个方法列出 Rust helper 单测（helper 覆盖不等于完整 handler 覆盖）。bridge 方法测试按表中精确用例名标注。
- macOS 原生观察：3/167 个方法；来源为 [CSP / custom protocol smoke 记录](wry-custom-protocol-plan.md#L42) 与 [roadmap smoke 记录](roadmap.md#L92)。
- Windows 原生观察：73/167 个方法；68 个来自 [run_api.py](../examples/windows-smoke/run_api.py) 的逐方法行为检查，另 5 个来自 [run_stream.py](../examples/windows-smoke/run_stream.py) 的大流检查。均以 Windows 任务在真机运行整套脚本的记录为准；其他方法不因此通过。
- “未观察”表示本次查阅的记录未提供该方法的原生观察证据，不代表功能失败。适用平台按类型声明中的 macOS/Windows 限定填写；不要求对明确的平台专属方法在另一平台补测。未发现逐方法行为断言时，也不把契约映射测试或通用代理测试代作覆盖。
