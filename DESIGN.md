# DeepSeek Harness GUI 后端设计选型

## 1. 文档状态与范围

本文记录当前已经确认的 Tauri 后端设计。实现范围以 Rust 后端、IPC、跨平台进程管理、网络探测、主题监听、日志和系统托盘为主，前端视觉与页面结构不在本文范围内。

本文中的接口名称、状态语义、超时、路径规则和平台行为均作为实现约束。后续实现应优先保持接口稳定，再完善内部实现。

## 2. 总体架构

后端采用“统一业务入口 + 单一 IPC 文件 + 编译期平台适配”的结构。

```text
Frontend
  ├─ invoke("shell")
  ├─ invoke("start_dsh")
  ├─ invoke("stop_dsh")
  ├─ invoke("connect_remote")
  ├─ invoke("open_logs")
  ├─ invoke("hide_to_tray")
  ├─ invoke("get_curr_theme")
  ├─ listen("send_curr_theme")
  └─ listen("dsh_exited")
          │
          ▼
src-tauri/src/ipc.rs
          │
          ▼
src-tauri/src/backend/mod.rs
  ├─ shell.rs
  ├─ harness.rs
  ├─ network.rs
  ├─ theme.rs
  ├─ logs.rs
  ├─ tray.rs
  └─ error.rs
          │
          ▼
src-tauri/src/platform/mod.rs
  ├─ unix.rs
  └─ windows.rs
```

架构约束：

- `main.rs` 只调用库入口，不承载业务逻辑。
- `lib.rs` 负责 Tauri Builder、插件注册、状态初始化、后台任务启动和应用生命周期接线。
- 所有自定义 Tauri commands 集中在 `ipc.rs`。
- `ipc.rs` 只做参数接收、业务调用和错误转换，不实现业务流程。
- `backend/mod.rs` 是后端统一业务 façade，对 `ipc.rs` 暴露业务函数。
- 通用业务逻辑全部位于 `backend`。
- 只有真正依赖操作系统的进程创建、进程树管理和终止逻辑位于 `platform`。
- `platform/mod.rs` 使用 `cfg` 在编译期选择 `unix.rs` 或 `windows.rs`。
- 平台选型使用静态分派，不提供运行时平台切换。
- Unix 和 Windows 平台实现提供相同的内部进程控制接口。
- 所有 IPC command 都是异步调用。
- 不共享状态的 IPC 可以并发执行。
- `start_dsh` 和 `stop_dsh` 共享同一个 dsh 生命周期，只允许串行改变该状态。

建议文件结构：

```text
src-tauri/src/
├── main.rs
├── lib.rs
├── ipc.rs
├── backend/
│   ├── mod.rs
│   ├── error.rs
│   ├── shell.rs
│   ├── harness.rs
│   ├── network.rs
│   ├── theme.rs
│   ├── logs.rs
│   └── tray.rs
└── platform/
    ├── mod.rs
    ├── unix.rs
    └── windows.rs
```

## 3. IPC 总览

最终公共 IPC 接口为：

```text
shell(request) -> ShellResult
start_dsh(port) -> address
stop_dsh() -> success
connect_remote(host, port) -> address
open_logs() -> success
hide_to_tray() -> success
get_curr_theme() -> theme
```

Rust 侧概念签名：

```rust
async fn shell(request: ShellRequest) -> Result<ShellResult, IpcError>;
async fn start_dsh(port: u16) -> Result<String, IpcError>;
async fn stop_dsh() -> Result<(), IpcError>;
async fn connect_remote(host: String, port: u16) -> Result<String, IpcError>;
async fn open_logs() -> Result<(), IpcError>;
async fn hide_to_tray() -> Result<(), IpcError>;
async fn get_curr_theme() -> Result<String, IpcError>;
```

以下能力只作为 Rust 内部函数存在，不注册为 IPC command：

```text
check_tcp
check_http
check_https
```

环境检查和 dsh 安装通过通用 `shell` IPC 完成，不提供独立的 `check_env` 或 `install_dsh` IPC。

## 4. 统一错误契约

所有普通业务 IPC 使用统一结构化错误：

```rust
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcError {
    pub code: String,
    pub message: String,
}
```

对应前端：

```typescript
interface IpcError {
  code: string;
  message: string;
}
```

错误契约：

- `code` 是稳定、机器可读的错误标识。
- `message` 是前端可以直接展示的用户可读文本。
- 前端根据 `code` 决定是否 toast、静默、重试或切换页面。
- 前端不得通过解析 `message` 判断错误类型。
- 内部模块保留具体错误类型和完整错误链。
- 只有 `ipc.rs` 将内部错误映射为 `IpcError`。
- 底层系统错误、路径、命令上下文和错误链写入日志，不默认暴露给前端。

稳定错误码至少覆盖：

```text
invalid_command
invalid_timeout
invalid_cwd
shell_spawn_failed
invalid_host
invalid_port
service_unavailable
port_occupied
operation_in_progress
dsh_already_running
process_not_running
dsh_spawn_failed
dsh_start_timeout
dsh_exited_early
theme_not_available
open_logs_failed
```

## 5. 通用 Shell IPC

### 5.1 职责

前端不直接调用 `@tauri-apps/plugin-shell`。所有短命令统一通过自定义 Rust `shell` IPC 执行。

典型调用：

```text
node -V
npm -V
dsh -V
npm install -g @deepseek-ai/dsh
```

前端环境检查流程：

1. 调用 `shell` 执行 `node -V`。
2. 调用 `shell` 执行 `npm -V`。
3. 调用 `shell` 执行 `dsh -V`。
4. Node 或 npm 不存在时，前端提示用户前往官网安装。
5. dsh 不存在时，前端显示安装按钮。
6. 安装按钮通过 `shell` 执行 `npm install -g @deepseek-ai/dsh`。
7. 安装结束后再次通过 `shell` 执行 `dsh -V`。
8. `dsh -V` 成功后认为 dsh 可以运行。

### 5.2 请求结构

```rust
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellRequest {
    pub command: String,
    pub cwd: Option<std::path::PathBuf>,
    pub timeout_ms: Option<i64>,
}
```

对应前端：

```typescript
interface ShellRequest {
  command: string;
  cwd?: string;
  timeoutMs?: number;
}
```

请求规则：

- `command` 是完整 shell 命令字符串。
- `command` 不能为空。
- 不提供独立的 `args`。
- 不提供自定义 `env`。
- 不提供 request ID。
- 首版不提供主动取消。
- `timeoutMs` 单位固定为毫秒。
- `timeoutMs` 省略时默认 30 秒。
- `timeoutMs <= 0` 返回 `invalid_timeout`。
- Rust 先用有符号整数接收并校验 `timeoutMs`，校验完成后再转换成 `Duration`。

### 5.3 Shell 解释器

各平台使用固定解释器：

```text
Windows       -> cmd.exe /D /S /C <command>
macOS/Linux   -> /bin/bash -c <command>
```

`command` 按目标系统解释器的语法执行，可以包含管道、重定向和条件执行。前端负责生成目标系统可识别的命令文本。

### 5.4 工作目录

`cwd` 规则：

- `cwd` 省略时使用用户 Home 目录。
- 绝对路径直接作为工作目录。
- 相对路径相对于用户 Home 目录解析。
- 解析后的目录必须存在。
- 解析后的路径必须是目录。
- Rust 不自动创建工作目录。
- 无效工作目录返回 `invalid_cwd`。

### 5.5 PATH

应用启动时调用一次 `fix-path-env-rs` 的 PATH 修复入口，且必须发生在启动任何 shell 或 dsh 子进程之前。

PATH 规则：

- 所有 shell 子进程继承修复后的应用环境。
- dsh 专用进程继承同一份修复后的 PATH。
- `ShellRequest` 不允许覆盖环境变量。
- 应用运行期间系统 PATH 的变化不会自动重新加载。
- PATH 发生变化后需要重启应用。
- PATH 修复失败时记录 error 日志，但不阻止 Tauri 应用启动。
- PATH 修复失败后，命令找不到时由对应 shell 或 dsh 操作返回具体错误。

### 5.6 返回结构

```rust
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub status: ShellStatus,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ShellStatus {
    Success,
    Failed,
    Timeout,
}
```

对应前端：

```typescript
interface ShellResult {
  exitCode: number | null;
  stdout: string;
  stderr: string;
  status: "success" | "failed" | "timeout";
}
```

`ShellResult` 不包含嵌套 `error` 字段。

状态映射：

```text
进程退出码为 0          -> status = success
进程退出码非 0          -> status = failed
进程无退出码异常结束     -> status = failed, exitCode = null
达到 timeoutMs          -> status = timeout, exitCode = null
```

命令已经启动后的正常退出、非零退出和超时都返回 `Ok(ShellResult)`。

以下情况通过外层 `Err(IpcError)` 返回：

- 请求参数非法。
- Home 目录无法确定。
- cwd 无效。
- shell 解释器无法创建。
- 子进程在建立有效执行结果前发生内部错误。

### 5.7 输出捕获

输出规则：

- 同时捕获 `stdout` 和 `stderr`。
- 两个流在进程运行期间并发读取，避免管道阻塞。
- 命令结束后一次性返回完整输出。
- 输出使用 UTF-8 lossy 解码。
- 非法 UTF-8 字节替换为 Unicode replacement character。
- 不返回 base64 或原始字节数组。
- `stdout` 和 `stderr` 不设置大小上限。
- 不提供截断标识。

### 5.8 并发

- 通用 shell IPC 不设置并发上限。
- 每个调用立即创建独立的受控进程树。
- shell 调用之间不共享业务状态。
- 每个调用独立计算超时。
- 每个调用独立收集输出和退出状态。

### 5.9 超时和进程树清理

达到 `timeoutMs` 后：

- `status` 固定为 `timeout`。
- `exitCode` 固定为 `null`。
- 必须终止 shell 解释器及其创建的完整进程树。
- Windows 通过对应 Job Object 终止整个进程树。
- Unix 向对应进程组发送 `SIGTERM`。
- Unix 在 `SIGTERM` 后最多等待 5 秒。
- 进程组仍未退出时发送 `SIGKILL`。
- Unix 调用的最晚返回时间可能达到 `timeoutMs + 5 秒`。
- 超时前已经读取到的 stdout/stderr 仍放入 `ShellResult`。

## 6. 平台进程管理

### 6.1 平台内部接口

`platform` 提供通用的受控进程抽象。其职责包括：

```text
spawn_managed
wait
try_wait
terminate_tree
build_shell_command
build_dsh_command
```

`ManagedProcess` 表示完整受控进程树，不只是直接子进程 PID。

平台实现必须支持：

- 异步等待进程退出。
- 查询进程是否已经退出。
- 捕获 stdout/stderr。
- 正常退出码读取。
- 无退出码状态表达。
- 完整进程树终止。
- shell timeout 清理。
- dsh 主动停止。
- 应用退出时的 dsh 清理。

### 6.2 Windows

Windows 平台实现位于 `platform/windows.rs`。

约束：

- 受控命令使用 Job Object。
- shell 每次调用拥有独立 Job Object。
- dsh 拥有独立、长生命周期 Job Object。
- Job Object 覆盖 shell/dsh 创建的后代进程。
- Job Object 配置关闭 handle 时终止其中的进程。
- shell timeout 通过终止对应 Job Object 清理进程树。
- `stop_dsh` 通过 dsh Job Object 终止进程树。
- Windows shell 使用 `cmd.exe`。
- Windows 平台实现负责基于修复后的 PATH 启动 dsh。

### 6.3 Unix

Unix 平台实现位于 `platform/unix.rs`，同时覆盖 macOS 和 Linux。

约束：

- 每个受控 shell 命令创建独立进程组。
- dsh 创建独立进程组。
- 终止操作向整个进程组发送信号。
- 优雅停止使用 `SIGTERM`。
- 强制停止使用 `SIGKILL`。
- Unix shell 使用 `/bin/bash`。

## 7. dsh 生命周期

### 7.1 单实例状态机

整个应用最多维护一个 dsh 实例。

```text
Stopped -> Starting -> Running -> Stopping -> Stopped
```

状态约束：

- `start_dsh` 和 `stop_dsh` 使用异步互斥保护生命周期变更。
- 任意时刻只能有一个生命周期变更操作。
- 冲突操作不排队，立即返回 `operation_in_progress`。
- `Running` 状态再次调用 `start_dsh` 返回 `dsh_already_running`。
- `Stopped` 状态调用 `stop_dsh` 返回 `process_not_running`。
- TCP/HTTP 探测和其他 IPC 不受 dsh 生命周期锁影响。
- shell IPC 与 dsh 生命周期独立。
- 后台退出监控必须核对进程身份，旧进程的退出通知不能覆盖新进程状态。

### 7.2 启动接口

```text
start_dsh(port: u16) -> Result<String, IpcError>
```

端口规则：

- IPC 使用 `u16`。
- `port == 0` 返回 `invalid_port`。
- 前端负责将表单字符串转换成数字。
- Rust 负责最终端口校验。

启动流程：

1. 获取 dsh 生命周期操作权。
2. 验证当前状态为 `Stopped`。
3. 将状态变为 `Starting`。
4. 检查 `127.0.0.1:PORT` 是否已经存在 TCP 服务。
5. 端口已被占用时恢复 `Stopped`，返回 `port_occupied`。
6. 通过平台受控进程接口启动 `dsh web --port PORT`。
7. 捕获并持续读取 dsh stdout/stderr。
8. 后台监控 dsh 是否提前退出。
9. 后端轮询 `http://127.0.0.1:PORT/`。
10. HTTP 就绪等待上限为 10 秒。
11. 只有 HTTP `2xx` 响应视为 WebUI 就绪。
12. 就绪后将状态变为 `Running`。
13. 返回 `http://127.0.0.1:PORT`。

前端调用 `start_dsh` 后直接等待 IPC 结果，不额外轮询端口。成功后使用 Rust 返回的地址跳转。

启动失败处理：

- dsh 创建失败时清理平台资源并恢复 `Stopped`。
- dsh 在就绪前退出时恢复 `Stopped`，返回 `dsh_exited_early`。
- 10 秒内没有得到 `2xx` 时终止刚启动的完整 dsh 进程树。
- 启动超时后恢复 `Stopped`，返回 `dsh_start_timeout`。
- dsh 启动阶段 stdout/stderr 写入本次启动日志。

### 7.3 停止接口

```text
stop_dsh() -> Result<(), IpcError>
```

停止流程：

1. 获取 dsh 生命周期操作权。
2. 确认当前维护着 dsh 进程。
3. 将状态变为 `Stopping`。
4. 调用当前平台的进程树终止实现。
5. 等待完整进程树退出。
6. 清理进程、Job Object、进程组和输出任务等资源。
7. 将状态变为 `Stopped`。
8. 返回成功。

平台停止语义：

- Windows 终止 dsh Job Object。
- Unix 向 dsh 进程组发送 `SIGTERM`。
- Unix 最多等待 5 秒。
- Unix 在等待后仍未退出时发送 `SIGKILL`。
- `stop_dsh` 只有在确认受控进程树已经退出后才返回成功。

### 7.4 异常退出

`Running` 状态下 dsh 自行退出时：

- 后台监控任务读取退出结果。
- 核对退出进程仍是当前维护的 dsh。
- 清理内部进程资源。
- 状态恢复为 `Stopped`。
- 向前端发送 `dsh_exited` 事件。
- 不自动重启 dsh。

事件契约：

```text
事件名：dsh_exited
```

事件 payload 至少能够让前端识别 dsh 已经退出；技术错误细节和 stdout/stderr 保留在日志中。前端自行决定是否 toast 或返回连接页面。

### 7.5 dsh 日志

- 捕获 dsh stdout。
- 捕获 dsh stderr。
- stdout/stderr 按行写入 `tauri-plugin-log`。
- 输出写入本次应用启动对应的日志目录。
- dsh 启动、就绪、停止、超时和退出码都写入日志。

### 7.6 应用退出保证

- 正常退出时保证执行 dsh 清理流程。
- 托盘“退出”先停止 dsh，再退出 Tauri 应用。
- Windows 额外依靠 Job Object handle 关闭语义清理进程树。
- Unix 在 GUI 被强制结束或收到 `SIGKILL` 时可能遗留 dsh 进程组。
- 不持久化 dsh PID。
- 下次启动时不根据旧 PID 自动终止进程。

## 8. 网络模块

### 8.1 内部职责

`backend/network.rs` 实现：

```text
check_tcp
check_http
check_https
```

这些函数只被 Rust 业务模块调用，不直接暴露给前端。

用途：

- `start_dsh` 使用 TCP 检查确认本地端口没有被占用。
- `start_dsh` 使用 HTTP 检查确认本地 WebUI 已就绪。
- `connect_remote` 使用 HTTPS/HTTP 检查确认远程服务可用。

### 8.2 connect_remote 接口

```text
connect_remote(host: String, port: u16) -> Result<String, IpcError>
```

输入规则：

- `host` 只接受 `localhost` 或语法合法的 IPv4。
- `localhost` 大小写不敏感。
- 返回地址中的 `localhost` 统一规范化为小写 `localhost`。
- IPv4 使用 Rust IPv4 解析器解析。
- 返回地址中的 IPv4 使用解析后的规范格式。
- 接受所有语法合法的 IPv4，包括特殊地址。
- 不接受普通域名。
- 不接受 IPv6。
- 不接受带协议的 host。
- 不接受带路径、查询参数、用户信息或端口的 host。
- `port` 类型为 `u16`。
- `port == 0` 返回 `invalid_port`。

### 8.3 协议探测

对同一个规范化后的 `HOST:PORT` 同时启动：

```text
https://HOST:PORT
http://HOST:PORT
```

探测规则：

- HTTPS 和 HTTP 异步并行探测。
- 每个探测拥有独立的 10 秒超时。
- HTTPS 使用 `reqwest` 默认 TLS、证书验证和重定向行为。
- 最终响应状态为 `2xx` 或 `3xx` 时视为远程服务可用。
- 最终响应状态为 `4xx` 或 `5xx` 时视为当前协议不可用。
- HTTPS 成功时立即返回 HTTPS 地址。
- HTTP 先成功时暂存 HTTP 结果，继续等待 HTTPS。
- HTTPS 失败或超时后，如果 HTTP 已成功，则返回 HTTP 地址。
- HTTPS 和 HTTP 都失败时返回 `service_unavailable`。
- 返回地址始终是规范化后的输入地址。
- 不返回重定向后的最终地址。

返回示例：

```text
connect_remote("LOCALHOST", 3000)
-> "https://localhost:3000" 或 "http://localhost:3000"

connect_remote("127.0.0.1", 3000)
-> "https://127.0.0.1:3000" 或 "http://127.0.0.1:3000"
```

## 9. 主题监听

### 9.1 文件位置

主题配置文件固定为：

```text
$HOME/.dsh/settings.yaml
```

读取字段：

```yaml
ui-theme: <string>
```

### 9.2 启动和监听流程

1. Tauri 启动后创建主题后台任务。
2. 每 3 秒检查 `settings.yaml` 是否存在。
3. 文件不存在时继续轮询。
4. 文件出现后停止轮询。
5. 立即读取并解析 `ui-theme`。
6. 成功解析后缓存主题字符串。
7. 向前端发送 `send_curr_theme` 事件。
8. 直接监听 `settings.yaml` 文件本身。
9. 文件修改时重新读取并解析。
10. 成功解析新值后更新缓存并再次发送事件。

监听只针对 `settings.yaml` 文件本身，不监听 `$HOME/.dsh/` 父目录。

### 9.3 数据契约

主题事件：

```text
send_curr_theme
payload：String
```

主题值规则：

- `ui-theme` 是字符串时原样发送。
- 后端不限制主题枚举值。
- YAML 解析失败时记录日志，不发送事件。
- `ui-theme` 缺失时记录日志，不发送事件。
- `ui-theme` 类型不是字符串时记录日志，不发送事件。
- 解析失败不会终止应用。
- 前端负责未知主题值的 fallback。

### 9.4 初始主题可靠性

后端缓存最近一次成功解析的主题。

```text
get_curr_theme() -> Result<String, IpcError>
```

前端初始化顺序：

1. 注册 `send_curr_theme` 事件监听。
2. 调用 `get_curr_theme()`。
3. 查询成功时立即应用当前主题。
4. 后续通过 `send_curr_theme` 接收变化。

当前尚未发现有效主题时，`get_curr_theme()` 返回 `theme_not_available`。

## 10. 日志

### 10.1 日志插件

- 使用 `tauri-plugin-log`。
- 日志插件在 Tauri Builder 中只注册一次。
- 当前模板中的重复日志插件注册在实现时删除。
- Rust 应用日志和 dsh stdout/stderr 使用同一日志系统。

### 10.2 本次启动日志目录

每次 Tauri 启动时创建独立日志目录：

```text
<Tauri 平台应用日志目录>/<秒级 Unix timestamp>/
```

示例：

```text
<AppLogDir>/1786884645/
```

规则：

- 使用 Tauri 平台标准应用日志目录作为父目录。
- 子目录名称直接使用秒级 Unix timestamp。
- `tauri-plugin-log` 的文件 target 指向该目录。
- 后端保存本次启动目录的绝对路径。
- 所有后端模块通过统一日志 facade 写入该目录。

### 10.3 查看日志

```text
open_logs() -> Result<(), IpcError>
```

行为：

- 使用 `tauri-plugin-opener` 打开本次启动日志目录。
- 打开成功返回 `Ok(())`。
- 打开失败记录完整错误并返回 `open_logs_failed`。

## 11. 系统托盘和窗口生命周期

### 11.1 主窗口关闭

- 用户点击主窗口关闭按钮时阻止默认退出。
- 主窗口被隐藏。
- dsh 保持运行。
- 前端调用 `hide_to_tray()` 时执行同样的隐藏行为。

### 11.2 托盘菜单

托盘右键菜单提供：

```text
显示
退出
```

“显示”行为：

1. 显示主窗口。
2. 将主窗口恢复到前台。
3. 聚焦主窗口。

“退出”行为：

1. 阻止重复退出流程。
2. 检查是否维护着 dsh。
3. dsh 运行时执行内部 `stop_dsh` 业务函数。
4. 完成进程树清理。
5. 结束 Tauri 应用。

退出流程由 Rust 控制，不依赖前端页面仍然存在。

## 12. Tauri 初始化顺序

`lib.rs::run()` 按以下顺序组织初始化职责：

1. 解析应用所需的平台标准目录。
2. 创建本次启动的 timestamp 日志目录。
3. 配置并注册一次 `tauri-plugin-log`。
4. 调用 `fix-path-env-rs` 修复应用 PATH。
5. 注册 `tauri-plugin-opener`。
6. 初始化 dsh 状态、主题缓存和本次日志目录状态。
7. 注册系统托盘及菜单事件。
8. 注册 `ipc.rs` 中的全部 commands。
9. 在 setup 阶段启动主题轮询/监听任务。
10. 进入 Tauri 事件循环。

PATH 修复失败只记录 error 日志，应用继续初始化。

## 13. Tauri 插件和权限

后端使用：

```text
tauri-plugin-log
tauri-plugin-opener
```

通用 Shell 由自定义 Rust IPC 实现，因此：

- 前端不依赖 `@tauri-apps/plugin-shell`。
- Rust 后端不依赖 `tauri-plugin-shell`。
- capability 不授予 `shell:*` 插件权限。
- shell 执行能力只通过自定义 `shell` command 暴露。

主题监听由 Rust 文件监听实现，不依赖前端文件 watch。

## 14. Rust 依赖职责

实现需要以下依赖类别：

```text
Tauri v2                      应用运行时、IPC、事件、窗口和托盘
tauri-plugin-log              文件日志
tauri-plugin-opener           打开本次日志目录
fix-path-env-rs               修复 GUI 应用 PATH
tokio                         异步进程、任务、IO、互斥和超时
reqwest                       HTTP/HTTPS 探测
serde / serde_json            IPC 序列化
serde_yaml                    settings.yaml 解析
notify                         跨平台文件监听
thiserror                      内部类型化错误
Unix 进程/信号依赖            进程组、SIGTERM、SIGKILL
Windows API 依赖              Job Object 和进程树管理
```

依赖版本和 feature 在实现阶段按当前官方文档确定。

## 15. 全局行为约束

- 前端只通过已定义 IPC 和事件与后端业务交互。
- 通用 shell 允许执行完整目标平台 shell 命令。
- shell 没有命令 allowlist。
- shell 不设置并发上限。
- shell 不设置输出大小上限。
- shell 首版不支持主动取消。
- dsh 始终由专用后端生命周期管理，不通过通用 shell 启动。
- dsh 始终作为单实例受控进程树。
- `start_dsh` 成功表示本地 WebUI 已返回 HTTP `2xx`，不是仅表示进程创建成功。
- `connect_remote` 成功表示对应规范化地址经过 HTTP/HTTPS 探测可用。
- 主题变化由后端主动推送，查询接口只补偿初始事件时序。
- 正常退出必须清理 dsh。
- Unix 强制终止 GUI 时可能遗留 dsh，这是当前版本接受的运行语义。
