# DeepSeek Harness GUI 后端设计选型

## 0. 需求

DSH（DeepSeek Harness）是一个使用命令 `npm install -g @deepseek-ai/dsh` 安装的应用，安装后，命令行中可使用使用命令 `dsh --profile web --port <PORT>` 启动一个 http web server，浏览器打开 127.0.0.1:PORT 即可访问 WEBUI。

DSH 应用有大量的访问网络、读写文件、执行命令行的权限。

本项目使用 Tauri 给这一应用包装一个壳子，让它变成 APP，同时保留原应用的所有权限。

## 1. 文档状态与范围

本文记录当前已经确认的 Tauri 应用设计。实现范围以 Rust 后端、IPC、跨平台进程管理、网络探测、前端主题监听、日志和系统托盘为主，前端视觉与页面结构不在本文范围内。

本文中的接口名称、状态语义、超时、路径规则和平台行为均作为实现约束。后续实现应优先保持接口稳定，再完善内部实现。

## 2. 总体架构

后端采用“统一业务入口 + 单一 IPC 文件 + 编译期平台适配”的结构；短命令由前端通过 `tauri-plugin-shell` 直接执行，主题配置由前端通过 `tauri-plugin-fs` 读取和监听，长生命周期的 dsh 由 Rust 后端统一管理。

```text
Frontend
  ├─ Command.create("node-version")
  ├─ Command.create("npm-version")
  ├─ Command.create("dsh-version")
  ├─ Command.create("npm-install-dsh-npmjs" / "npm-install-dsh-npmmirror")
  ├─ invoke("start_dsh")
  ├─ invoke("stop_dsh")
  ├─ invoke("connect_remote")
  ├─ invoke("open_logs")
  ├─ invoke("hide_to_tray")
  ├─ fs.exists/readTextFile/watch("$HOME/.dsh/settings.yaml")
  └─ listen("dsh_exited")
          │
          ├──────────────────────┬─────────────────────┐
          ▼                      ▼                     ▼
src-tauri/src/ipc.rs       tauri-plugin-shell     tauri-plugin-fs
          │                 (短命令)               (主题配置)
          ▼
src-tauri/src/backend/mod.rs
  ├─ harness.rs
  ├─ network.rs
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
- `lib.rs` 负责 Tauri Builder、插件注册、PATH 初始化、状态初始化、后台任务启动和应用生命周期接线。
- 所有自定义 Tauri commands 集中在 `ipc.rs`。
- `ipc.rs` 只做参数接收、业务调用和错误转换，不实现业务流程。
- 短命令不再经过自定义 `shell` IPC，由前端直接调用 `tauri-plugin-shell`。
- 主题配置不进入自定义 IPC 或 Rust `backend`，由前端通过 `tauri-plugin-fs` 读取和监听。
- `backend/mod.rs` 是后端统一业务 façade，对 `ipc.rs` 暴露业务函数。
- 通用业务逻辑全部位于 `backend`。
- 只有真正依赖操作系统的 dsh 进程创建、进程树管理和终止逻辑位于 `platform`。
- `platform/mod.rs` 使用 `cfg` 在编译期选择 `unix.rs` 或 `windows.rs`。
- 平台选型使用静态分派，不提供运行时平台切换。
- Unix 和 Windows 平台实现提供相同的内部进程控制接口。
- 所有自定义 IPC command 都是异步调用。
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
│   ├── harness.rs
│   ├── network.rs
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
start_dsh(port) -> address
stop_dsh() -> success
connect_remote(protocol, host, port) -> address
open_logs() -> success
hide_to_tray() -> success
```

Rust 侧概念签名：

```rust
async fn start_dsh(port: u16) -> Result<String, IpcError>;
async fn stop_dsh() -> Result<(), IpcError>;
async fn connect_remote(protocol: String, host: String, port: u16) -> Result<String, IpcError>;
async fn open_logs() -> Result<(), IpcError>;
async fn hide_to_tray() -> Result<(), IpcError>;
```

以下能力只作为 Rust 内部函数存在，不注册为 IPC command：

```text
check_tcp
check_url
```

环境检查和 dsh 安装由前端直接通过 `tauri-plugin-shell` 完成；主题读取和监听由前端直接通过 `tauri-plugin-fs` 完成。不提供自定义 `shell`、`check_env`、`install_dsh` 或主题 IPC。

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
- `tauri-plugin-shell` 和 `tauri-plugin-fs` 的调用错误由前端单独处理，不纳入自定义 `IpcError` 契约。
- 底层系统错误、路径、命令上下文和错误链写入日志，不默认暴露给前端。

稳定错误码至少覆盖：

```text
invalid_host
invalid_protocol
invalid_port
service_unavailable
port_occupied
operation_in_progress
dsh_already_running
process_not_running
dsh_spawn_failed
dsh_start_timeout
dsh_exited_early
open_logs_failed
```

## 5. 前端短命令执行

### 5.1 职责与边界

短时运行且不需要后端维护生命周期的命令，由前端直接通过 `@tauri-apps/plugin-shell` 执行，不再提供自定义 `shell` IPC。

当前只覆盖以下操作：

```text
node -V
npm -V
dsh -V
npm install -g --verbose @deepseek-ai/dsh --registry=https://registry.npmjs.org
npm install -g --verbose @deepseek-ai/dsh --registry=https://registry.npmmirror.org
```

边界约束：

- 版本检查和 dsh 安装不进入 `ipc.rs`、`backend` 或 `platform`。
- 前端使用 `Command.create()` 和参数数组，不拼接完整 shell 命令字符串。
- 不显式调用 Windows `cmd.exe` 或 Unix shell，不支持管道、重定向、条件执行和 shell 变量展开。
- `dsh --profile web --port <PORT>` 是长生命周期受控进程，始终通过 `start_dsh` 和 `stop_dsh` 管理，不允许前端通过 shell 插件启动。
- shell 插件进程不复用 dsh 的 Job Object、Unix 进程组或生命周期状态机。

### 5.2 PATH 修复时机

应用进入 `lib.rs::run()` 后尽早调用一次 `fix_path_env::fix()`，且必须发生在任何 shell 插件命令或 dsh 子进程启动之前。

PATH 规则：

- PATH 修复作用于当前 Tauri Rust 进程的环境。
- 后续由 `tauri-plugin-shell` 和 dsh 平台层创建的子进程继承修复后的 PATH。
- 前端不读取、不拼接也不覆盖 PATH。
- PATH 修复失败时记录 error 日志，但不阻止 Tauri 应用启动。
- PATH 修复失败后，命令找不到时由对应插件调用或 dsh 操作返回具体错误。
- 应用运行期间系统 PATH 的变化不会自动重新加载。
- 安装 Node.js、npm 或修改 PATH 后需要重启应用。
- `fix-path-env-rs` 只修复 PATH，不提供 shell 解释器语义，也不会在 Windows 上把 `npm` 或 `dsh` 自动扩展为 `.cmd`。

### 5.3 跨平台命令映射和 capability

前端只使用平台无关的逻辑命令名；capability 按目标平台将逻辑命令映射到实际程序和固定参数。

```text
逻辑命令名                Windows 实际程序   macOS/Linux 实际程序   固定参数
node-version              node.exe           node                    -V
npm-version               npm.cmd            npm                     -V
dsh-version               dsh.cmd            dsh                     -V
npm-install-dsh-npmjs     npm.cmd            npm                     install -g --verbose @deepseek-ai/dsh --registry=https://registry.npmjs.org
npm-install-dsh-npmmirror npm.cmd            npm                     install -g --verbose @deepseek-ai/dsh --registry=https://registry.npmmirror.org
```

约束：

- Windows 必须显式配置 `npm.cmd` 和 `dsh.cmd`，不能依赖裸命令名自动解析 command shim。
- `node.exe` 可以通过修复后的 PATH 查找；Unix 命令使用无扩展名入口。
- Windows 与 macOS/Linux 使用平台限定的 capability，但保持相同的逻辑命令名。
- capability 只允许上述五种固定操作，不向前端开放任意程序或任意参数。
- `node-version`、`npm-version` 和 `dsh-version` 授予 `shell:allow-execute`。
- `npm-install-dsh-npmjs` 和 `npm-install-dsh-npmmirror` 授予 `shell:allow-spawn`。
- 原 `npm-install-dsh` 已拆分为双 registry 变体，前端以单一 `useInstallDsh(mirror)` composable 封装二者。
- 前端传给 `Command.create()` 的名称是 capability 中的逻辑命令名，不是操作系统实际程序名。

### 5.4 `execute`：版本检查

`node -V`、`npm -V` 和 `dsh -V` 输出少且应立即结束，统一使用 `execute()`，等待进程退出后一次性读取 `code`、`signal`、`stdout` 和 `stderr`。

```typescript
import { Command } from "@tauri-apps/plugin-shell";

const nodeResult = await Command.create("node-version").execute();
const npmResult = await Command.create("npm-version").execute();
const dshResult = await Command.create("dsh-version").execute();
```

结果规则：

- `code === 0` 表示对应命令可执行。
- `code !== 0` 表示命令已启动但执行失败，前端可以展示 `stderr`。
- Promise rejection 表示权限拒绝、程序未找到或进程创建失败等插件调用错误。
- 前端根据退出码判断结果，不解析版本输出文本来判断命令是否存在。

### 5.5 `spawn`：dsh 安装

`npm install -g --verbose @deepseek-ai/dsh --registry=<REGISTRY>` 运行时间相对更长且输出量大，统一使用 `spawn()`，在进程运行期间持续消费 stdout/stderr。registry 固定为官方源或 npmmirror 源，对应两个固定 scope 命令，前端通过 `useInstallDsh(mirror)` composable 选择。

```typescript
import { Command } from "@tauri-apps/plugin-shell";

const command = Command.create("npm-install-dsh-npmjs", [
  "install",
  "-g",
  "--verbose",
  "@deepseek-ai/dsh",
  "--registry=https://registry.npmjs.org",
]);

command.stdout.on("data", appendInstallOutput);
command.stderr.on("data", appendInstallOutput);
command.on("close", ({ code }) => finishInstallation(code));
command.on("error", reportInstallError);

await command.spawn();
```

安装流程：

1. 前端调用 `npm-install-dsh-npmjs` 或 `npm-install-dsh-npmmirror` 的 `spawn()`。
2. 注册并持续消费 stdout/stderr，避免大量输出只在结束后一次性显示。
3. `close.code === 0` 时再次调用 `dsh-version` 的 `execute()`。
4. `dsh-version` 成功后认为 dsh 已安装并可以运行。
5. 安装进程只属于当前前端操作，不写入 Rust dsh 生命周期状态，也不作为应用后台服务维护。

### 5.6 前端环境检查流程

1. 使用 `node-version.execute()` 执行 `node -V`。
2. 使用 `npm-version.execute()` 执行 `npm -V`。
3. 使用 `dsh-version.execute()` 执行 `dsh -V`。
4. Node 或 npm 不存在时，前端提示用户前往官网安装。
5. dsh 不存在时，前端显示安装按钮。
6. 安装按钮使用 `npm-install-dsh-npmjs` 或 `npm-install-dsh-npmmirror` 的 `spawn()` 执行安装并实时展示输出。
7. 安装成功后再次使用 `dsh-version.execute()` 验证。
8. `dsh-version` 退出码为 `0` 后认为 dsh 可以运行。

### 5.7 插件调用语义

- shell 插件调用错误不经过自定义 `IpcError`，由前端单独归一化和展示。
- shell 插件命令之间不共享业务状态，可以并发执行；前端应避免重复触发同一个安装操作。
- 不为版本检查或安装命令提供自定义 Rust 超时、Job Object 或 Unix 进程组清理。
- `execute()` 只用于立即返回且输出有限的版本检查。
- `spawn()` 只用于需要实时消费大量输出的安装操作。
- 前端不通过 shell 插件执行任意用户输入命令。

## 6. 平台进程管理

### 6.1 模块职责与 seam

`platform` 是后端唯一允许直接接触 dsh 操作系统进程 API 的模块。它隐藏以下差异：

- Windows Job Object 与 Unix 进程组的创建和持有。
- Windows 进程树终止与 Unix 信号发送。
- 平台原生进程 handle、PID、PGID 和退出状态读取。
- Windows dsh command shim 与 Unix 可执行文件的启动形式。
- 平台资源的释放和退出进程回收。

`backend/harness.rs` 不得出现以下内容：

```text
#[cfg(windows)]
#[cfg(unix)]
Windows handle
Job Object
PID / PGID
SIGTERM / SIGKILL
cmd.exe / bash
```

平台模块不负责以下业务规则：

- 前端 shell 插件命令的 capability、参数或结果处理。
- dsh 单实例状态机。
- dsh 启动前端口检查。
- dsh HTTP 就绪探测。
- dsh 事件发送。
- stdout/stderr 文本解码和日志格式。
- `IpcError` 构造。

这些规则继续由前端或通用 `backend` 模块负责。

### 6.2 文件和编译期选型

```text
src-tauri/src/platform/
├── mod.rs
├── windows.rs
└── unix.rs
```

只有 `platform/mod.rs` 负责 `cfg` 选型：

```rust
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
use unix as current;
#[cfg(windows)]
use windows as current;

pub(crate) use current::ManagedProcess;
```

约束：

- Windows 构建只编译 `windows.rs`。
- macOS/Linux 构建只编译 `unix.rs`。
- `backend` 只依赖 `platform/mod.rs` 暴露的类型和函数。
- 不定义运行时平台枚举。
- 不使用 `Box<dyn PlatformProcess>`。
- 不提供动态平台切换。
- `windows.rs` 和 `unix.rs` 必须实现相同的方法集合和行为契约。

### 6.3 `mod.rs` 共享数据类型

`platform/mod.rs` 定义平台无关的输入、输出和错误类型。平台原生 handle 只存在于当前平台实现的私有字段中。

概念接口：

```rust
use std::{io, time::Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProcessExit {
    pub exit_code: Option<i32>,
}

pub(crate) struct SpawnedProcess {
    pub process: ManagedProcess,
    pub stdout: tokio::process::ChildStdout,
    pub stderr: tokio::process::ChildStderr,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum PlatformError {
    #[error("failed to build dsh command: {source}")]
    BuildCommand {
        #[source]
        source: io::Error,
    },

    #[error("failed to spawn dsh process: {source}")]
    Spawn {
        #[source]
        source: io::Error,
    },

    #[error("failed to control dsh process tree: {source}")]
    Control {
        #[source]
        source: io::Error,
    },

    #[error("failed to wait for dsh process: {source}")]
    Wait {
        #[source]
        source: io::Error,
    },
}
```

接口语义：

- `ProcessExit.exit_code` 只表达系统退出结果，不映射业务错误。
- 正常退出码映射为 `Some(code)`。
- 无数字退出码的终止映射为 `None`。
- `PlatformError` 是内部错误，不直接序列化给前端。
- `backend/error.rs` 负责将 `PlatformError` 转换为业务错误。
- `ipc.rs` 再将业务错误转换为 `IpcError`。
- 平台函数不使用 `String` 作为错误类型。
- 生产代码不使用 `unwrap()` 或 `expect()` 处理平台操作。

`SpawnedProcess` 中的 stdout/stderr 管道在创建成功时必须存在。平台实现必须在 spawn 前配置：

```text
stdin  = null
stdout = piped
stderr = piped
```

stdout/stderr 的读取、UTF-8 lossy 解码和业务日志输出不属于平台层。

### 6.4 `mod.rs` 对内接口

`platform/mod.rs` 对 `backend` 暴露以下静态接口：

```rust
pub(crate) fn spawn_dsh(
    port: u16,
) -> Result<SpawnedProcess, PlatformError> {
    current::spawn_dsh(port)
}
```

`ManagedProcess` 在 `windows.rs` 和 `unix.rs` 中分别定义，但必须提供一致的方法：

```rust
impl ManagedProcess {
    pub(crate) fn try_wait(
        &self,
    ) -> Result<Option<ProcessExit>, PlatformError>;

    pub(crate) async fn wait(
        &self,
    ) -> Result<ProcessExit, PlatformError>;

    pub(crate) async fn terminate_tree(
        &self,
        grace_period: Duration,
    ) -> Result<ProcessExit, PlatformError>;
}
```

方法契约：

#### `try_wait`

- 不阻塞当前任务。
- 进程仍在运行时返回 `Ok(None)`。
- 进程已经退出时返回缓存的 `Ok(Some(ProcessExit))`。
- 可以重复调用。
- 不消费退出结果。

#### `wait`

- 异步等待进程退出。
- 进程已经退出时立即返回缓存结果。
- 可以被多个任务同时调用。
- 所有等待者必须得到同一个 `ProcessExit`。
- 不要求调用者持有可变引用。
- 不允许在等待期间持有 dsh 状态机互斥锁。

#### `terminate_tree`

- 终止该 `ManagedProcess` 对应的完整进程树。
- 已经退出时直接返回缓存的退出结果。
- 可以与另一个任务中的 `wait()` 并发执行。
- 可以被重复调用。
- 并发终止请求只执行一次实际终止流程，其他调用等待同一个结果。
- 返回前必须完成直接子进程回收。
- Unix 使用传入的 `grace_period` 控制 `SIGTERM` 到 `SIGKILL` 的等待时间。
- Windows 保留相同参数，但终止 Job Object 时不执行 Unix 信号等待。

### 6.5 `ManagedProcess` 通用并发框架

`ManagedProcess` 是可克隆的轻量控制 handle。内部可使用 `Arc` 共享平台资源和退出状态，但不得复制操作系统资源的所有权语义。

统一内部框架：

```text
spawn_dsh
  ├─ 创建平台进程树容器
  ├─ 创建并启动直接子进程
  ├─ 取出 stdout/stderr
  ├─ 启动唯一 reaper/supervisor
  ├─ 返回 SpawnedProcess
  │    ├─ ManagedProcess
  │    ├─ stdout
  │    └─ stderr
  └─ supervisor 等待退出并缓存 ProcessExit
```

`ManagedProcess` 的共享状态至少包含：

```text
平台控制资源
退出结果缓存
退出通知
终止流程互斥状态
```

并发约束：

- 只有一个 supervisor/reaper 负责等待直接子进程，避免重复 `wait`。
- supervisor 必须持有直接子进程的可变所有权。
- `wait()` 通过共享退出通知等待 supervisor 结果。
- `try_wait()` 读取共享退出缓存。
- `terminate_tree()` 使用独立的平台控制资源发起终止，不需要取得子进程可变所有权。
- `terminate_tree()` 之后仍由同一个 supervisor 回收直接子进程。
- 后台 dsh 退出监控持有一个 `ManagedProcess` clone。
- dsh 状态持有另一个 `ManagedProcess` clone，用于主动停止。
- 最后一个 handle 被释放时关闭平台资源。

Windows Job handle 与 Unix PGID 都通过私有字段封装。`backend` 不得依赖其具体类型。

### 6.6 Windows dsh 框架

`platform/windows.rs` 负责 Windows dsh 命令构造、Job Object、进程 handle 和进程树终止。

结构框架：

```rust
use std::sync::Arc;

pub(crate) struct ManagedProcess {
    inner: Arc<WindowsProcess>,
}

struct WindowsProcess {
    job: OwnedJobHandle,
    process_id: u32,
    exit_state: SharedExitState,
    termination_state: TerminationState,
}

pub(super) fn spawn_dsh(
    port: u16,
) -> Result<SpawnedProcess, PlatformError>;
```

上述私有辅助类型表示职责，不要求跨平台共享具体定义：

- `OwnedJobHandle` 独占 Job Object handle，并在 `Drop` 时关闭 handle。
- `SharedExitState` 缓存 `ProcessExit` 并通知所有等待任务。
- `TerminationState` 保证终止流程只执行一次。
- Windows 原生 handle 不暴露到 `platform/mod.rs` 之外。

#### 6.6.1 Windows dsh 构造

`spawn_dsh` 构造受控命令：

```text
dsh --profile web --port <PORT>
```

约束：

- 使用修复后的 PATH 解析 npm 全局安装的 dsh 命令入口。
- Windows command shim 的解析和启动细节只存在于 `windows.rs`。
- dsh 的直接启动进程及其 Node 后代进程必须属于同一个 Job Object。
- dsh 的 cwd 不由前端提供。
- stdout/stderr 必须通过 `SpawnedProcess` 返回。

#### 6.6.2 Job Object 创建流程

每个 dsh 实例创建一个 Job Object：

1. 创建 Job Object。
2. 设置 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`。
3. 创建 stdout/stderr pipe。
4. 创建目标进程。
5. 在目标进程创建工作负载后代之前将其关联到 Job Object。
6. 启动唯一 supervisor 等待直接子进程退出。
7. 返回 `SpawnedProcess`。

如果 Job Object 创建、配置或进程关联失败：

- 终止已经创建的直接子进程。
- 关闭已经创建的 Job handle 和 pipe handle。
- 回收直接子进程。
- 返回 `PlatformError::Spawn` 或 `PlatformError::Control`。
- 不返回部分初始化的 `ManagedProcess`。

#### 6.6.3 Windows 终止流程

`terminate_tree` 执行：

1. 检查共享退出缓存。
2. 进程已退出时直接返回缓存结果。
3. 取得终止流程执行权。
4. 调用 Job Object 终止能力结束整个进程树。
5. 等待 supervisor 回收直接子进程。
6. 返回 supervisor 缓存的 `ProcessExit`。

Windows 的 `grace_period` 不改变 Job Object 终止流程。业务层仍使用相同的方法签名。

`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` 同时作为资源释放时的清理保证。正常业务流程仍显式调用 `terminate_tree()`，不依赖 `Drop` 作为主要控制路径。

### 6.7 Unix dsh 框架

`platform/unix.rs` 同时覆盖 macOS 和 Linux，负责 dsh 命令构造、进程组创建和信号终止。

结构框架：

```rust
use std::sync::Arc;

pub(crate) struct ManagedProcess {
    inner: Arc<UnixProcess>,
}

struct UnixProcess {
    leader_pid: u32,
    process_group_id: i32,
    exit_state: SharedExitState,
    termination_state: TerminationState,
}

pub(super) fn spawn_dsh(
    port: u16,
) -> Result<SpawnedProcess, PlatformError>;
```

上述私有辅助类型表示职责：

- `leader_pid` 标识直接子进程，由唯一 supervisor 回收。
- `process_group_id` 标识完整受控进程组。
- `SharedExitState` 缓存直接子进程退出结果并通知等待任务。
- `TerminationState` 保证信号终止流程只执行一次。

#### 6.7.1 Unix dsh 构造

`spawn_dsh` 构造：

```text
program = dsh
args    = --profile web --port PORT
env     = 继承应用启动时修复后的环境
stdin   = null
stdout  = piped
stderr  = piped
```

约束：

- 通过修复后的 PATH 查找 `dsh`。
- dsh 的 cwd 不由前端提供。
- dsh 必须成为新进程组的 leader。
- dsh 创建的所有后代进程继承该进程组。

#### 6.7.2 Unix 进程组创建流程

每个 dsh 实例创建独立进程组：

1. 创建 stdout/stderr pipe。
2. 配置子进程在执行目标程序前进入新的进程组。
3. 启动目标进程。
4. 记录直接子进程 PID 和 PGID。
5. 启动唯一 supervisor 等待直接子进程退出。
6. 返回 `SpawnedProcess`。

进程组必须在 dsh 创建后代进程之前建立，避免后代进程脱离受控进程组。

#### 6.7.3 Unix 终止流程

`terminate_tree(grace_period)` 执行：

1. 检查共享退出缓存。
2. 进程已退出时直接返回缓存结果。
3. 取得终止流程执行权。
4. 向整个进程组发送 `SIGTERM`。
5. 等待 supervisor 退出通知，最长等待 `grace_period`。
6. 在期限内退出时返回缓存结果。
7. 到期后向整个进程组发送 `SIGKILL`。
8. 等待 supervisor 回收直接子进程。
9. 返回缓存的 `ProcessExit`。

信号目标必须是整个进程组，不得只向 `leader_pid` 发送信号。

当目标进程组已经不存在时，终止流程继续等待或读取 supervisor 的退出结果，不把“进程已经消失”当作新的业务失败。

Unix `Drop` 不发送信号。正常停止、dsh 启动超时和应用正常退出必须显式调用 `terminate_tree()`。

### 6.8 通用调用流程

#### 6.8.1 dsh 启动

`backend/harness.rs` 使用平台接口：

```text
检查端口
  -> platform::spawn_dsh(port)
  -> 启动 stdout/stderr 日志任务
  -> 保存 ManagedProcess clone 和进程 generation
  -> 启动 wait() 退出监控
  -> 执行 HTTP 2xx 就绪探测
  -> 成功后进入 Running
```

平台层不执行 TCP/HTTP 检查，也不发送 Tauri 事件。

#### 6.8.2 dsh 停止

```text
取得当前 ManagedProcess
  -> terminate_tree(5 秒)
  -> 等待 stdout/stderr 日志任务完成
  -> 核对 generation
  -> 清理状态
  -> 进入 Stopped
```

### 6.9 资源、安全与实现约束

- Windows 原生 handle 使用拥有所有权的 RAII wrapper。
- handle wrapper 必须实现正确的 `Send`/`Sync` 约束。
- Unix PID/PGID 使用明确的内部类型，避免相互混用。
- 平台原生错误统一转换为包含 `std::io::Error` 的 `PlatformError`。
- 所有 `unsafe` 代码限制在 `windows.rs` 或 `unix.rs` 的最小私有函数中。
- 每个 `unsafe` 块必须记录其 handle、PID、指针和生命周期不变量。
- 不把原始 handle 或裸指针存入 Tauri managed state。
- 不在持有同步 Mutex guard 时执行 `.await`。
- 不在 `wait()` 期间独占 dsh 生命周期锁。
- 不把 stdout/stderr 读取放入平台终止互斥区。
- 短命令不进入平台层。
- dsh 进程自然退出、主动停止和启动超时终止最终都由唯一 supervisor 回收。
- 平台资源初始化必须具备失败回滚，不能泄漏半初始化进程或 handle。
- `ManagedProcess` 的业务身份由 `harness.rs` 的 generation 管理，平台层只负责系统进程身份。

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
- 前端 shell 插件命令与 dsh 生命周期独立。
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
6. 通过平台受控进程接口启动 `dsh --profile web --port <PORT>`。
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
check_url
```

这些函数只被 Rust 业务模块调用，不直接暴露给前端。

用途：

- `start_dsh` 使用 TCP 检查确认本地端口没有被占用。
- `start_dsh` 使用 `check_url("http", ...)` 检查确认本地 WebUI 已就绪。
- `connect_remote` 使用 `check_url(协议, ...)` 检查确认远程服务可用。

### 8.2 connect_remote 接口

```text
connect_remote(protocol: String, host: String, port: u16) -> Result<String, IpcError>
```

输入规则：

- `protocol` 只接受小写 `http` 或 `https`。
- 其他 `protocol` 值返回 `invalid_protocol`。
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

只对用户指定协议构造的单个地址发起探测：

```text
http://HOST:PORT     （protocol = http 时）
https://HOST:PORT    （protocol = https 时）
```

探测规则：

- 按 `protocol` 选择 `http` 或 `https` 单路探测，不并行、不回退。
- 探测拥有独立的 10 秒超时。
- HTTPS 使用 `reqwest` 默认 TLS、证书验证和重定向行为。
- 最终响应状态仅为 `2xx` 时视为服务可用。
- `3xx`、`4xx`、`5xx` 及其他状态均视为当前协议不可用。
- 服务可用时返回 `Ok(address)`，否则返回 `service_unavailable`。
- 返回地址始终是规范化后的输入地址。
- 不返回重定向后的最终地址。

返回示例：

```text
connect_remote("http", "LOCALHOST", 3000)
-> "http://localhost:3000"

connect_remote("https", "127.0.0.1", 3000)
-> "https://127.0.0.1:3000"
```

## 9. 前端主题读取与监听

### 9.1 职责与文件位置

主题只影响本地前端 UI，由前端直接通过 `@tauri-apps/plugin-fs` 读取和监听，不进入 `ipc.rs`、Rust `backend`、自定义事件或后端状态缓存。

主题配置文件固定为：

```text
$HOME/.dsh/settings.yaml
```

前端使用 `BaseDirectory.Home` 访问相对路径 `.dsh/settings.yaml`，由 Tauri 在 Windows、macOS 和 Linux 上解析用户 Home 目录；不手工拼接平台路径。需要绝对路径时，可以使用 `@tauri-apps/api/path` 的 `homeDir()` 和 `join()`。

只读取以下字段：

```yaml
ui-theme:
  preference: dark
```

其他 YAML 字段全部忽略。

### 9.2 主题值契约

前端主题类型固定为：

```typescript
type ThemePreference = "light" | "dark" | "system";
```

主题值规则：

- `ui-theme.preference` 为 `light`、`dark` 或 `system` 时使用该值。
- `ui-theme` 或 `preference` 缺失时使用 `system`。
- `preference` 类型错误或值不受支持时使用 `system`。
- YAML 解析失败或文件读取失败时记录前端错误，并保留最近一次有效主题。
- 尚未成功读取过主题时，前端初始主题固定为 `system`。
- 不提供 `get_curr_theme` IPC、`send_curr_theme` 事件或 Rust 主题缓存。

### 9.3 启动和监听流程

1. 本地前端启动时立即应用 `system`，避免等待文件期间主题未定义。
2. 每 3 秒调用 `exists(".dsh/settings.yaml", { baseDir: BaseDirectory.Home })`。
3. 文件不存在时继续轮询。
4. 文件出现后停止轮询。
5. 先调用 `watch()` 监听该文件，再执行首次读取，避免“读取完成到监听注册”之间丢失修改。
6. `watch()` 使用 `delayMs: 300`，由 `tauri-plugin-fs` 合并连续文件事件。
7. 使用 `readTextFile()` 读取文件，并通过前端 YAML 解析库解析 `ui-theme.preference`。
8. 每次得到有效或 fallback 主题后更新前端主题状态并应用到 UI。
9. 文件修改后重新读取并解析；不根据文件事件 payload 推断主题值。
10. 页面或应用销毁时调用 `watch()` 返回的取消监听函数。

当前版本不处理文件开始监听后又被删除的情况。监听只针对 `settings.yaml` 文件本身，不递归监听 `$HOME/.dsh/`。

### 9.4 前端调用示例

```typescript
import { BaseDirectory, exists, readTextFile, watch } from "@tauri-apps/plugin-fs";
import { parse } from "yaml";

const settingsPath = ".dsh/settings.yaml";
const homeOptions = { baseDir: BaseDirectory.Home };

function parseTheme(contents: string): "light" | "dark" | "system" {
  const preference = parse(contents)?.["ui-theme"]?.preference;
  return preference === "light" || preference === "dark" || preference === "system" ? preference : "system";
}
```

业务层负责封装轮询、监听、读取和错误处理，页面组件只订阅前端主题状态，不直接操作文件系统。

### 9.5 文件系统权限

capability 只允许主窗口对单个主题配置文件执行存在检查、文本读取和监听：

```json
{
  "identifier": "fs:allow-exists",
  "allow": [{ "path": "$HOME/.dsh/settings.yaml" }]
}
```

```json
{
  "identifier": "fs:allow-read-text-file",
  "allow": [{ "path": "$HOME/.dsh/settings.yaml" }]
}
```

```json
{
  "identifier": "fs:allow-watch",
  "allow": [{ "path": "$HOME/.dsh/settings.yaml" }]
}
```

权限约束：

- 不授予 `$HOME/**` 或 `$HOME/.dsh/**` 的宽泛读取权限。
- 不授予写入、创建、删除、目录遍历或递归监听权限。
- 主题 capability 只授予承载本地包装器 UI 的可信 WebView，不授予可能加载远程 dsh WebUI 的窗口或 WebView。
- 前端可以读取该 YAML 文件的完整文本，因此该文件不得存放不应暴露给本地前端的密钥或凭据。

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

1. 调用 `fix-path-env-rs` 修复应用 PATH，且不得在此前启动任何子进程。
2. 解析应用所需的平台标准目录。
3. 创建本次启动的 timestamp 日志目录。
4. 配置并注册一次 `tauri-plugin-log`。
5. 注册 `tauri-plugin-shell`。
6. 注册启用 `watch` feature 的 `tauri-plugin-fs`。
7. 注册 `tauri-plugin-opener`。
8. 初始化 dsh 状态和本次日志目录状态。
9. 注册系统托盘及菜单事件。
10. 注册 `ipc.rs` 中的全部自定义 commands。
11. 进入 Tauri 事件循环。

PATH 修复失败只记录 error 日志，应用继续初始化。前端只能在 Tauri 初始化完成后调用 shell 和 fs 插件，因此 shell 插件进程和 dsh 子进程继承修复后的 PATH，主题文件路径由 fs/path API 按目标平台解析。

## 13. Tauri 插件和权限

后端使用：

```text
tauri-plugin-log
tauri-plugin-shell
tauri-plugin-fs
tauri-plugin-opener
```

短命令由前端直接使用 `@tauri-apps/plugin-shell`：

- Rust 后端注册一次 `tauri-plugin-shell`。
- 前端依赖 `@tauri-apps/plugin-shell`，不再调用自定义 `shell` command。
- capability 按平台将相同逻辑命令名映射到实际的 `node`、`npm` 和 `dsh` 入口。
- capability 只授予版本检查和 dsh 安装所需的 `shell:allow-execute`、`shell:allow-spawn` 及固定 scope。
- 不授予任意命令、任意参数、`shell:allow-kill` 或 `shell:allow-stdin-write`。

主题配置由前端直接使用 `@tauri-apps/plugin-fs`：

- Rust 后端注册启用 `watch` feature 的 `tauri-plugin-fs`。
- 前端使用 `BaseDirectory.Home` 或 `@tauri-apps/api/path` 解析跨平台 Home 路径。
- capability 仅向可信本地前端授予 `$HOME/.dsh/settings.yaml` 的 `fs:allow-exists`、`fs:allow-read-text-file` 和 `fs:allow-watch`。
- 不授予主题文件的写入、删除、目录遍历或宽泛 Home 目录访问权限。
- 不提供自定义主题 IPC、Rust 主题状态或 Rust 主题事件。

## 14. 依赖职责

Rust 依赖：

```text
Tauri v2                      应用运行时、IPC、事件、窗口和托盘
tauri-plugin-log              文件日志
tauri-plugin-shell            前端短命令执行和安装输出流
tauri-plugin-fs               前端主题文件读取和防抖监听，启用 watch feature
tauri-plugin-opener           打开本次日志目录
fix-path-env-rs               修复 GUI 应用 PATH
tokio                         异步进程、任务、IO、互斥和超时
reqwest                       HTTP/HTTPS 探测
serde / serde_json            IPC 序列化
thiserror                      内部类型化错误
Unix 进程/信号依赖            进程组、SIGTERM、SIGKILL
Windows API 依赖              Job Object 和进程树管理
```

前端依赖：

```text
@tauri-apps/plugin-shell       调用固定短命令
@tauri-apps/plugin-fs          检查、读取和监听主题配置文件
@tauri-apps/api               使用 path API 处理跨平台路径
yaml                          解析 settings.yaml
```

文件监听由 `tauri-plugin-fs` 的 `watch` feature 内部提供，项目不再维护 Rust 主题解析或监听依赖。

依赖版本和 feature 在实现阶段按当前官方文档确定。

## 15. 全局行为约束

- 前端通过已定义 IPC 和事件访问后端业务，通过 `tauri-plugin-shell` 访问固定短命令，通过 `tauri-plugin-fs` 访问固定主题配置文件。
- 不提供自定义 `shell` IPC。
- shell 插件 capability 只允许固定的版本检查和 dsh 安装操作。
- 前端使用参数数组，不执行任意完整 shell 命令字符串。
- 版本检查使用 `execute()`，dsh 安装使用 `spawn()` 并实时消费 stdout/stderr。
- shell 插件调用不进入 dsh 生命周期管理，也不提供自定义 Rust 超时或完整进程树清理。
- dsh 始终由专用后端生命周期管理，不通过前端 shell 插件启动。
- dsh 始终作为单实例受控进程树。
- `start_dsh` 成功表示本地 WebUI 已返回 HTTP `2xx`，不是仅表示进程创建成功。
- `connect_remote` 成功表示对应规范化地址经过用户指定协议（http 或 https）探测可用。
- 主题由可信本地前端通过 `tauri-plugin-fs` 读取和监听，缺失或无效主题值回退到 `system`。
- 主题功能不使用自定义 IPC、Rust 状态缓存或自定义 Tauri 事件。
- 正常退出必须清理 dsh。
- Unix 强制终止 GUI 时可能遗留 dsh，这是当前版本接受的运行语义。
