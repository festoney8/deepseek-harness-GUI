use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};

use crate::logs::{self, Session};
use crate::os::{self, Harness};
use crate::protocol::{Phase, Snapshot, EVENT_RUNTIME_STATE};

pub const PORT_START: u16 = 3080;
pub const PORT_END: u16 = 5080;
const START_TIMEOUT: Duration = Duration::from_secs(120);
const POLL_INTERVAL: Duration = Duration::from_millis(500);
/// 安装与版本查询共用的 npm registry 常量
const REGISTRY_OFFICIAL: &str = "https://registry.npmjs.org";
const REGISTRY_MIRROR: &str = "https://registry.npmmirror.com";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Intent {
    CheckEnv,
    CheckVersion,
    Install { mirror: bool },
    Start,
}

struct SharedRuntime {
    snapshot: Mutex<Snapshot>,
    /// 当前日志会话（worker 创建后挂载，供 open_log_dir 取目录）
    session: Mutex<Option<Arc<Session>>>,
    /// 环境检查轮次：每次触发自增，任务写回前校验，旧轮次结果丢弃
    env_gen: AtomicU64,
    /// 版本检查轮次：同上
    version_gen: AtomicU64,
}

type Shared = Arc<SharedRuntime>;

/// 前端意图入口：worker 持有接收端，命令侧持有发送端。
pub struct Supervisor {
    tx: Sender<Intent>,
    rx: Mutex<Option<Receiver<Intent>>>,
    shared: Shared,
}

impl Supervisor {
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            tx,
            rx: Mutex::new(Some(rx)),
            shared: Arc::new(SharedRuntime {
                snapshot: Mutex::new(Snapshot::default()),
                session: Mutex::new(None),
                env_gen: AtomicU64::new(0),
                version_gen: AtomicU64::new(0),
            }),
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        self.shared
            .snapshot
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// 当前日志会话目录（`<base>/logs/<timestamp>/`），会话未创建时为 None
    pub fn log_dir(&self) -> Option<PathBuf> {
        self.shared
            .session
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|session| session.dir().to_path_buf())
    }

    fn send(&self, intent: Intent) {
        let _ = self.tx.send(intent);
    }

    pub fn check_env(&self) {
        self.send(Intent::CheckEnv);
    }

    pub fn check_version(&self) {
        self.send(Intent::CheckVersion);
    }

    pub fn install(&self, mirror: bool) {
        self.send(Intent::Install { mirror });
    }

    pub fn start(&self) {
        self.send(Intent::Start);
    }

    pub fn spawn_worker(&self, app: AppHandle, base: PathBuf) {
        let rx = self
            .rx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
            .expect("worker already spawned");
        let shared = self.shared.clone();
        std::thread::Builder::new()
            .name("supervisor".into())
            .spawn(move || worker_loop(app, rx, shared, base))
            .expect("spawn supervisor thread");
    }
}

fn emit_state(
    app: &AppHandle,
    shared: &SharedRuntime,
    phase: Phase,
    port: Option<u16>,
    detail: &str,
    elapsed: Option<u64>,
) {
    let mut state = shared
        .snapshot
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    state.phase = phase;
    state.port = port;
    state.detail = detail.to_string();
    state.elapsed = elapsed;
    let snapshot = state.clone();
    drop(state);
    let _ = app.emit(EVENT_RUNTIME_STATE, snapshot);
}

/// 失败态快捷入口：统一走 Failed 阶段、无 elapsed。
fn emit_fail(app: &AppHandle, shared: &SharedRuntime, port: Option<u16>, detail: &str) {
    emit_state(app, shared, Phase::Failed, port, detail, None);
}

/// 线程内更新快照并推送：lock → f(&mut Snapshot) → clone → emit runtime-state。
fn update_snapshot(app: &AppHandle, shared: &SharedRuntime, f: impl FnOnce(&mut Snapshot)) {
    let mut state = shared
        .snapshot
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    f(&mut state);
    let snapshot = state.clone();
    drop(state);
    let _ = app.emit(EVENT_RUNTIME_STATE, snapshot);
}

/// 发起单个检查任务：spawn_blocking 执行同步检查（blocking 池，不占 worker），
/// catch_unwind 兜底 panic（按失败处理），写回前校验轮次（旧轮结果丢弃）。
fn spawn_check(
    app: AppHandle,
    shared: Shared,
    session: Arc<Session>,
    gen: u64,
    gen_of: fn(&SharedRuntime) -> &AtomicU64,
    run: impl FnOnce(&Arc<Session>) -> Option<String> + Send + 'static,
    apply: impl Fn(&mut Snapshot, Option<String>) + Send + 'static,
) {
    tauri::async_runtime::spawn_blocking(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(&session)))
            .ok()
            .flatten();
        if gen_of(&shared).load(Ordering::SeqCst) == gen {
            update_snapshot(&app, &shared, |snapshot| apply(snapshot, result));
        }
    });
}

fn worker_loop(app: AppHandle, rx: Receiver<Intent>, shared: Shared, base: PathBuf) {
    match logs::cleanup_old(&base, Duration::from_secs(7 * 86_400)) {
        Ok(removed) => log::debug!("cleaned up {removed} old log sessions"),
        Err(error) => log::warn!("failed to clean up old log sessions: {error}"),
    }

    let Some(session) = create_session(&base) else {
        log::error!("cannot create log session at {}", base.display());
        emit_fail(&app, &shared, None, "无法创建日志目录，请检查磁盘空间或权限。");
        return;
    };
    logs::attach_session(session.clone());
    // 挂载当前会话，open_log_dir 命令据此定位日志目录
    *shared
        .session
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(session.clone());

    log::info!("supervisor worker started");
    // 启动即自动检查环境与版本，填充格子
    check_env(&app, &shared, &session);
    check_version(&app, &shared, &session);
    emit_state(&app, &shared, Phase::Idle, None, "就绪", None);

    loop {
        match rx.recv() {
            Err(_) => {
                log::info!("supervisor channel closed, worker exiting");
                return;
            }
            Ok(Intent::CheckEnv) => {
                log::debug!("intent: check env");
                check_env(&app, &shared, &session);
                emit_state(&app, &shared, Phase::Idle, None, "就绪", None);
            }
            Ok(Intent::CheckVersion) => {
                log::debug!("intent: check version");
                check_version(&app, &shared, &session);
                emit_state(&app, &shared, Phase::Idle, None, "就绪", None);
            }
            Ok(Intent::Install { mirror }) => {
                let registry = if mirror { REGISTRY_MIRROR } else { REGISTRY_OFFICIAL };
                log::info!("intent: install dsh (registry={registry})");
                install_dsh(&app, &shared, &session, registry);
            }
            Ok(Intent::Start) => {
                let Some(port) = find_port() else {
                    log::error!("no free port in {PORT_START}-{PORT_END}");
                    emit_fail(&app, &shared, None, "端口 3080-5080 均被占用，请检查占用程序。");
                    continue;
                };
                log::info!("starting harness on port {port}");
                let mut harness = match Harness::spawn(port, &base) {
                    Ok(harness) => harness,
                    Err(e) => {
                        log::error!("failed to spawn harness: {e}");
                        emit_fail(&app, &shared, Some(port), &format!("启动失败：{e}"));
                        continue;
                    }
                };
                emit_state(
                    &app,
                    &shared,
                    Phase::Starting,
                    Some(port),
                    "正在启动 DeepSeek Harness…",
                    Some(0),
                );

                if let Some(out) = harness.stdout() {
                    spawn_pipe_reader(out, session.clone(), "[stdout] ", None);
                }
                if let Some(err) = harness.stderr() {
                    spawn_pipe_reader(err, session.clone(), "[stderr] ", None);
                }

                // 就绪轮询（进程存活 + HTTP 可响应，120 秒超时）
                let started = Instant::now();
                let ready = 'poll: {
                    loop {
                        match harness.try_wait() {
                            Ok(Some(status)) => {
                                log::error!("harness exited early with code {:?}", status.code());
                                os::kill_active();
                                emit_fail(
                                    &app,
                                    &shared,
                                    Some(port),
                                    &format!(
                                        "进程提前退出（退出码 {:?}），请查看日志。",
                                        status.code()
                                    ),
                                );
                                break 'poll false;
                            }
                            Ok(None) => {}
                            Err(e) => {
                                log::error!("failed to read harness status: {e}");
                                emit_fail(
                                    &app,
                                    &shared,
                                    Some(port),
                                    &format!("无法读取进程状态：{e}"),
                                );
                                break 'poll false;
                            }
                        }
                        if http_ok(port) {
                            log::info!("harness responds on http://127.0.0.1:{port}");
                            break 'poll true;
                        }
                        if started.elapsed() > START_TIMEOUT {
                            log::error!(
                                "harness start timed out after {}s",
                                START_TIMEOUT.as_secs()
                            );
                            os::kill_active();
                            emit_fail(&app, &shared, Some(port), "启动超时（120 秒），请查看日志。");
                            break 'poll false;
                        }
                        emit_state(
                            &app,
                            &shared,
                            Phase::Starting,
                            Some(port),
                            "正在启动 DeepSeek Harness…",
                            Some(started.elapsed().as_secs()),
                        );
                        std::thread::sleep(POLL_INTERVAL);
                    }
                };
                if !ready {
                    continue;
                }

                // Ready：监控进程，异常退出恢复窗口
                emit_state(&app, &shared, Phase::Ready, Some(port), "服务已就绪", None);
                log::info!("harness ready on http://127.0.0.1:{port}");
                loop {
                    match harness.try_wait() {
                        Ok(Some(status)) => {
                            let code = status.code();
                            os::kill_active();
                            if code == Some(0) {
                                log::info!("harness exited with code 0, quitting");
                                shutdown(&app);
                                return;
                            }
                            log::warn!("harness exited with code {code:?}");
                            emit_state(
                                &app,
                                &shared,
                                Phase::Failed,
                                Some(port),
                                &format!("DeepSeek Harness 已退出（退出码 {code:?}）。"),
                                None,
                            );
                            show_main(&app);
                            break;
                        }
                        Ok(None) => {}
                        Err(_) => {}
                    }
                    std::thread::sleep(POLL_INTERVAL);
                }
            }
        }
    }
}

struct CommandSpec<'a> {
    display: &'a str,
    program: &'a str,
    args: &'a [&'a str],
}

/// 以指定 registry 全局安装/更新 dsh，成功则刷新版本信息
fn install_dsh(app: &AppHandle, shared: &Shared, session: &Arc<Session>, registry: &str) {
    log::info!("installing dsh globally via npm (registry={registry})");
    emit_state(
        app,
        shared,
        Phase::Installing,
        None,
        "正在安装/更新 DeepSeek Harness…",
        None,
    );
    let display = format!("npm install --verbose -g @deepseek-ai/dsh --registry={registry}");
    let args: [&str; 6] = [
        "install",
        "--verbose",
        "-g",
        "@deepseek-ai/dsh",
        "--registry",
        registry,
    ];
    let ok = match run_cmd_streamed(
        session,
        CommandSpec {
            display: &display,
            program: "npm",
            args: &args,
        },
    ) {
        Ok((status, _)) => status.success(),
        Err(error) => {
            log::error!("dsh install command failed: {error}");
            false
        }
    };
    if ok {
        log::info!("dsh installed successfully");
        check_version(app, shared, session);
        emit_state(app, shared, Phase::Idle, None, "就绪", None);
    } else {
        log::error!("dsh install failed");
        emit_fail(app, shared, None, "安装失败，请查看日志。");
    }
}

fn check_env(app: &AppHandle, shared: &Shared, session: &Arc<Session>) {
    let gen = shared.env_gen.fetch_add(1, Ordering::SeqCst).wrapping_add(1);
    update_snapshot(app, shared, |snapshot| {
        snapshot.node = None;
        snapshot.npm = None;
    });
    spawn_check(
        app.clone(),
        shared.clone(),
        session.clone(),
        gen,
        |s| &s.env_gen,
        |session| {
            let version = stream_capture(
                session,
                CommandSpec {
                    display: "node -v",
                    program: "node",
                    args: &["-v"],
                },
            );
            log::info!("env check node: {version:?}");
            version
        },
        |snapshot, version| snapshot.node = version,
    );
    spawn_check(
        app.clone(),
        shared.clone(),
        session.clone(),
        gen,
        |s| &s.env_gen,
        |session| {
            let version = stream_capture(
                session,
                CommandSpec {
                    display: "npm -v",
                    program: "npm",
                    args: &["-v"],
                },
            );
            log::info!("env check npm: {version:?}");
            version
        },
        |snapshot, version| snapshot.npm = version,
    );
}

/// 版本查询 HTTP 请求超时：官方源与镜像源各一次，网络异常时快速失败
const VERSION_FETCH_TIMEOUT: Duration = Duration::from_secs(10);

/// 从指定 registry 的 JSON API 查询 dsh 最新版本（`/latest` 响应的 `version` 字段）。
/// 直连 HTTP 不经过 npm 代理配置（npm config proxy），属预期行为差异。
fn fetch_remote_version(registry: &str) -> Option<String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(VERSION_FETCH_TIMEOUT)
        .build();
    let url = format!("{registry}/@deepseek-ai/dsh/latest");
    let body = agent.get(&url).call().ok()?.into_string().ok()?;
    serde_json::from_str::<serde_json::Value>(&body)
        .ok()?
        .get("version")?
        .as_str()
        .map(String::from)
}

fn check_version(app: &AppHandle, shared: &Shared, session: &Arc<Session>) {
    let gen = shared.version_gen.fetch_add(1, Ordering::SeqCst).wrapping_add(1);
    update_snapshot(app, shared, |snapshot| {
        snapshot.remote = None;
        snapshot.remote_mirror = None;
        snapshot.local = None;
        snapshot.version_error = false;
        snapshot.version_checked = false;
    });
    // 官方源：附带置位 version_error / version_checked（安装按钮与兜底文案只依赖官方源结果）
    spawn_check(
        app.clone(),
        shared.clone(),
        session.clone(),
        gen,
        |s| &s.version_gen,
        |_session| {
            let remote = fetch_remote_version(REGISTRY_OFFICIAL);
            log::info!("dsh version check official: {remote:?}");
            remote
        },
        |snapshot, version| {
            let failed = version.is_none();
            snapshot.remote = version;
            snapshot.version_error = failed;
            snapshot.version_checked = true;
        },
    );
    spawn_check(
        app.clone(),
        shared.clone(),
        session.clone(),
        gen,
        |s| &s.version_gen,
        |_session| {
            let remote = fetch_remote_version(REGISTRY_MIRROR);
            log::info!("dsh version check mirror: {remote:?}");
            remote
        },
        |snapshot, version| snapshot.remote_mirror = version,
    );
    // 本地检测改用 `dsh -V`：安装不完整时该命令报错而非输出版本号，
    // 因此 local 能同时反映"已安装"与"安装完整"两个状态。
    spawn_check(
        app.clone(),
        shared.clone(),
        session.clone(),
        gen,
        |s| &s.version_gen,
        |session| {
            let local = stream_capture(
                session,
                CommandSpec {
                    display: "dsh -V",
                    program: "dsh",
                    args: &["-V"],
                },
            )
            .and_then(|out| parse_local_version(&out));
            log::info!("dsh local version: {local:?}");
            local
        },
        |snapshot, version| snapshot.local = version,
    );
}

fn stream_capture(session: &Arc<Session>, command: CommandSpec<'_>) -> Option<String> {
    match run_cmd_streamed(session, command) {
        Ok((status, output)) if status.success() => Some(output.trim().to_string()),
        _ => None,
    }
}

fn create_session(base: &Path) -> Option<Arc<Session>> {
    match Session::create(base) {
        Ok(session) => Some(Arc::new(session)),
        Err(error) => {
            log::error!("failed to create log session: {error}");
            None
        }
    }
}

fn run_cmd_streamed(
    session: &Arc<Session>,
    command: CommandSpec<'_>,
) -> std::io::Result<(ExitStatus, String)> {
    session.log_harness(&format!("[cmd] {}", command.display));
    log::debug!("executing: {}", command.display);

    let mut child = os::build_command(command.program, command.args).spawn()?;
    let collected = Arc::new(Mutex::new(String::new()));
    if let Some(stdout) = child.stdout.take() {
        spawn_pipe_reader(stdout, session.clone(), "[stdout] ", Some(collected.clone()));
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_pipe_reader(stderr, session.clone(), "[stderr] ", None);
    }
    let status = child.wait()?;
    let stdout = collected
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    Ok((status, stdout))
}

/// 从 `dsh -V` 输出解析版本号：`v0.1.0-rc.6` / `0.1.0-rc.6` 均归一为 `0.1.0-rc.6`。
fn parse_local_version(out: &str) -> Option<String> {
    let version = out.trim().strip_prefix('v').unwrap_or(out.trim());
    (!version.is_empty()).then(|| version.to_string())
}

fn spawn_pipe_reader<R: Read + Send + 'static>(
    reader: R,
    session: Arc<Session>,
    prefix: &'static str,
    collect: Option<Arc<Mutex<String>>>,
) {
    std::thread::spawn(move || {
        for line in BufReader::new(reader).lines().map_while(Result::ok) {
            let clean = sanitize(&line);
            session.log_harness(&format!("{prefix}{clean}"));
            if clean.is_empty() {
                continue;
            }
            if let Some(collected) = &collect {
                let mut collected = collected.lock().unwrap_or_else(|error| error.into_inner());
                collected.push_str(&clean);
                collected.push('\n');
            }
        }
    });
}

/// 去掉控制字符（保留换行与制表符），避免日志被当作终端/HTML 内容注入。
fn sanitize(line: &str) -> String {
    line.chars()
        .filter(|&c| c == '\n' || c == '\t' || c >= ' ')
        .collect()
}

fn port_free(port: u16) -> bool {
    TcpStream::connect_timeout(
        &SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(200),
    )
    .is_err()
}

fn find_port() -> Option<u16> {
    (PORT_START..=PORT_END).find(|&p| port_free(p))
}

/// 就绪探测：GET / 读到 HTTP 状态行即认为服务可响应。
fn http_ok(port: u16) -> bool {
    let Ok(mut s) = TcpStream::connect_timeout(
        &SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(500),
    ) else {
        return false;
    };
    let _ = s.set_read_timeout(Some(Duration::from_millis(500)));
    if s.write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut buf = [0u8; 64];
    let Ok(n) = s.read(&mut buf) else {
        return false;
    };
    String::from_utf8_lossy(&buf[..n]).starts_with("HTTP/")
}

/// 唯一关闭入口：先终止 harness 进程树，再退出应用。
/// KILL_ON_JOB_CLOSE 兜底：任何遗漏路径下句柄随进程关闭，OS 回收整个进程树。
pub fn shutdown(app: &AppHandle) {
    log::info!("application shutdown requested");
    os::kill_active();
    app.exit(0);
}

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}
