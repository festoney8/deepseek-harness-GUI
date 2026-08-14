# DeepSeek Harness GUI

将 [deepseek-harness](https://github.com/deepseek-ai/deepseek-harness)（`@deepseek-ai/dsh`）封装为 Windows 桌面应用的极薄壳层：不弹命令行，隐藏启动 `dsh --profile web`，以 iframe 嵌入其 WebUI。

## 开发

前置条件：Node.js、pnpm、Rust 工具链（rustup + MSVC）、WebView2 运行时。

```bash
pnpm install
pnpm tauri dev      # 开发模式（前端热更新 + 调试构建）
pnpm build:installer   # NSIS 安装包，产物在 src-tauri/target/release/bundle/nsis/
pnpm build:portable    # 免安装单文件 exe，产物在 src-tauri/target/release/bundle/portable/
```

## 行为说明

- 启动流程：检测 `node`/`npx` → 查找空闲端口（3080 起，依次回退至 5090）→ 隐藏命令行执行 `npx --yes @deepseek-ai/dsh --profile web --host 127.0.0.1 --port <port>` → 轮询就绪（进程存活 + HTTP 可响应，120 秒超时）→ 以 iframe 展示 WebUI。
- 进程树通过 Windows Job Object（`KILL_ON_JOB_CLOSE`）托管：退出应用时整个 harness 进程树一并终止，端口随即释放；任何异常退出路径由句柄回收兜底。
- 关闭按钮 / `Alt+F4` 弹出「退出 / 最小化到托盘 / 取消」；托盘左键恢复窗口，右键菜单含「显示 / 退出」；托盘退出立即生效，不再确认。
- 单实例：重复启动只恢复并聚焦已有窗口。
- harness 以退出码 0 自行结束时 GUI 一并退出；非零退出时恢复窗口并显示故障页。
- WebUI 禁止嵌入（`X-Frame-Options` / CSP `frame-ancestors`）时本项目会阻断并提示，不修改上游响应头。

## 日志

每次运行一个会话目录：

```
%LOCALAPPDATA%\com.festoney8.deepseek-harness-gui\logs\YYYY-MM-DD_HH-mm-ss-<pid>\
  harness.log   # dsh 进程原始输出（不脱敏）
  gui.log       # 壳自身日志（含时间戳）
```

启动时自动清理 14 天前的会话目录。日志目录创建失败会阻断启动。

## 发布门槛

公开发布前必须完成：

- Authenticode 签名（exe 与 NSIS 安装包），否则 Windows SmartScreen 会拦截。
- 确认 harness 仅监听 `127.0.0.1`（`netstat -ano | findstr <port>`），未暴露 `0.0.0.0` / `[::]`。启动命令已强制 `--host 127.0.0.1`，仍建议发布前抽样验证。

## 已知风险与取舍

- 退出为立即终止进程树，harness 正在进行的写入可能不完整。
- 完全信任 harness 自身的 localhost Web 安全（不附加代理或认证层）。
- 单会话日志不滚动，长期挂托盘运行可能产生大文件（14 天清理仅在下次启动时执行）。
- `npx` 始终解析 `@deepseek-ai/dsh` 的 latest 版本，行为随上游发布而变化。
- 无自动化测试，回归依赖发布前人工验收清单（托盘、端口释放、关闭交互、外链行为）。
- 假设目标机器已安装 WebView2；缺失时应用无法启动且无法显示错误页。

## 实现边界

- Rust：`lib.rs`（桌面集成）、`runtime.rs`（supervisor 状态机）、`logs.rs`（会话日志）。
- 前端：`App.vue`（壳层）、`TitleBar.vue`（自定义标题栏）、`StatusView.vue`（启动/故障页）、`WebUiView.vue`（iframe）、`useRuntime.ts`（状态与意图）。
- iframe 无 sandbox、无任何 Tauri IPC；前端仅保留 `core:default` 与 `opener:default` 权限。
