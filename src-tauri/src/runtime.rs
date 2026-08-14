use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

use crate::logs::{self, Session};
use crate::process::{self, Harness};

pub const PORT_START: u16 = 3080;
pub const PORT_END: u16 = 5080;
const START_TIMEOUT: Duration = Duration::from_secs(120);
const POLL_INTERVAL: Duration = Duration::from_millis(500);
/// UI 输出缓冲上限，超出丢弃最旧保留最新
const OUTPUT_LIMIT: usize = 1_048_576;

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    #[default]
    Idle,
    Installing,
    Starting,
    Ready,
    Failed,
}

/// 三格面板 + 终端的完整状态
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub phase: Phase,
    pub port: Option<u16>,
    pub detail: String,
    pub elapsed: Option<u64>,
    /// 格子一：node / npm 版本（None = 未检测到）
    pub node: Option<String>,
    pub npm: Option<String>,
    /// 格子二：远端 / 本地 dsh 版本（local None = 未安装；remote None = 获取失败）
    pub remote: Option<String>,
    pub local: Option<String>,
    pub version_error: bool,
    /// 版本检查是否已完成（false = 尚未检查，前端显示“检查中”）
    pub version_checked: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Intent {
    CheckEnv,
    CheckVersion,
    Install,
    Start,
}

struct SharedRuntime {
    snapshot: Mutex<Snapshot>,
    output: Mutex<String>,
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
                output: Mutex::new(String::new()),
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

    /// 完整命令行输出缓冲（stdout+stderr 合并）
    pub fn output(&self) -> String {
        self.shared
            .output
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
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

    pub fn install(&self) {
        self.send(Intent::Install);
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
    let _ = app.emit("runtime-state", snapshot);
}

fn worker_loop(app: AppHandle, rx: Receiver<Intent>, shared: Shared, base: PathBuf) {
    shared
        .output
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clear();
    let _ = app.emit("harness-output-reset", ());
    let _ = logs::cleanup_old(&base, Duration::from_secs(14 * 86_400));

    let Some(session) = create_session(&base) else {
        emit_state(
            &app,
            &shared,
            Phase::Failed,
            None,
            "无法创建日志目录，请检查磁盘空间或权限。",
            None,
        );
        return;
    };

    // 启动即自动检查环境与版本，填充格子
    check_env(&app, &shared, &session);
    check_version(&app, &shared, &session);
    emit_state(&app, &shared, Phase::Idle, None, "就绪", None);

    loop {
        match rx.recv() {
            Err(_) => return,
            Ok(Intent::CheckEnv) => {
                check_env(&app, &shared, &session);
                emit_state(&app, &shared, Phase::Idle, None, "就绪", None);
            }
            Ok(Intent::CheckVersion) => {
                check_version(&app, &shared, &session);
                emit_state(&app, &shared, Phase::Idle, None, "就绪", None);
            }
            Ok(Intent::Install) => {
                emit_state(
                    &app,
                    &shared,
                    Phase::Installing,
                    None,
                    "正在安装/更新 DeepSeek Harness…",
                    None,
                );
                let ok = match run_cmd_streamed(
                    &app,
                    &shared,
                    &session,
                    CommandSpec {
                        display: "npm install --verbose -g @deepseek-ai/dsh",
                        program: "cmd",
                        args: &[
                            "/C",
                            "npm",
                            "install",
                            "--verbose",
                            "-g",
                            "@deepseek-ai/dsh",
                        ],
                    },
                ) {
                    Ok((status, _)) => status.success(),
                    Err(_) => false,
                };
                if ok {
                    check_version(&app, &shared, &session);
                    emit_state(&app, &shared, Phase::Idle, None, "就绪", None);
                } else {
                    emit_state(
                        &app,
                        &shared,
                        Phase::Failed,
                        None,
                        "安装失败，请查看终端输出。",
                        None,
                    );
                }
            }
            Ok(Intent::Start) => {
                let Some(port) = find_port() else {
                    emit_state(
                        &app,
                        &shared,
                        Phase::Failed,
                        None,
                        "端口 3080-5080 均被占用，请检查占用程序。",
                        None,
                    );
                    continue;
                };
                let mut harness = match Harness::spawn(port, &base) {
                    Ok(harness) => harness,
                    Err(e) => {
                        emit_state(
                            &app,
                            &shared,
                            Phase::Failed,
                            Some(port),
                            &format!("启动失败：{e}"),
                            None,
                        );
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
                    spawn_pipe_reader(
                        out,
                        app.clone(),
                        session.clone(),
                        shared.clone(),
                        "[stdout] ",
                        None,
                    );
                }
                if let Some(err) = harness.stderr() {
                    spawn_pipe_reader(
                        err,
                        app.clone(),
                        session.clone(),
                        shared.clone(),
                        "[stderr] ",
                        None,
                    );
                }

                // 就绪轮询（进程存活 + HTTP 可响应，120 秒超时）
                let started = Instant::now();
                let ready = 'poll: {
                    loop {
                        match harness.try_wait() {
                            Ok(Some(status)) => {
                                process::kill_active();
                                emit_state(
                                    &app,
                                    &shared,
                                    Phase::Failed,
                                    Some(port),
                                    &format!(
                                        "进程提前退出（退出码 {:?}），请查看终端输出。",
                                        status.code()
                                    ),
                                    None,
                                );
                                break 'poll false;
                            }
                            Ok(None) => {}
                            Err(e) => {
                                emit_state(
                                    &app,
                                    &shared,
                                    Phase::Failed,
                                    Some(port),
                                    &format!("无法读取进程状态：{e}"),
                                    None,
                                );
                                break 'poll false;
                            }
                        }
                        if http_ok(port) {
                            break 'poll true;
                        }
                        if started.elapsed() > START_TIMEOUT {
                            process::kill_active();
                            emit_state(
                                &app,
                                &shared,
                                Phase::Failed,
                                Some(port),
                                "启动超时（120 秒），请查看终端输出。",
                                None,
                            );
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
                session.log_gui(&format!("harness ready on http://127.0.0.1:{port}"));
                loop {
                    match harness.try_wait() {
                        Ok(Some(status)) => {
                            let code = status.code();
                            process::kill_active();
                            if code == Some(0) {
                                session.log_gui("harness exited with code 0, quitting");
                                shutdown(&app);
                                return;
                            }
                            session.log_gui(&format!("harness exited with code {code:?}"));
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

fn check_env(app: &AppHandle, shared: &Shared, session: &Arc<Session>) {
    let node = stream_capture(
        app,
        shared,
        session,
        CommandSpec {
            display: "node -v",
            program: "node",
            args: &["-v"],
        },
    );
    let npm = stream_capture(
        app,
        shared,
        session,
        CommandSpec {
            display: "npm -v",
            program: "cmd",
            args: &["/C", "npm", "-v"],
        },
    );
    let mut snapshot = shared
        .snapshot
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    snapshot.node = node;
    snapshot.npm = npm;
}

fn check_version(app: &AppHandle, shared: &Shared, session: &Arc<Session>) {
    let remote = stream_capture(
        app,
        shared,
        session,
        CommandSpec {
            display: "npm view @deepseek-ai/dsh version",
            program: "cmd",
            args: &["/C", "npm", "view", "@deepseek-ai/dsh", "version"],
        },
    );
    let local_output = stream_capture(
        app,
        shared,
        session,
        CommandSpec {
            display: "npm list -g @deepseek-ai/dsh version",
            program: "cmd",
            args: &["/C", "npm", "list", "-g", "@deepseek-ai/dsh", "version"],
        },
    )
    .unwrap_or_default();
    let local = parse_local_version(&local_output);
    let version_error = remote.is_none();
    let mut snapshot = shared
        .snapshot
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    snapshot.remote = remote;
    snapshot.local = local;
    snapshot.version_error = version_error;
    snapshot.version_checked = true;
}

fn stream_capture(
    app: &AppHandle,
    shared: &Shared,
    session: &Arc<Session>,
    command: CommandSpec<'_>,
) -> Option<String> {
    match run_cmd_streamed(app, shared, session, command) {
        Ok((status, output)) if status.success() => Some(output.trim().to_string()),
        _ => None,
    }
}

fn create_session(base: &Path) -> Option<Arc<Session>> {
    Some(Arc::new(Session::create(base, std::process::id()).ok()?))
}

fn run_cmd_streamed(
    app: &AppHandle,
    shared: &Shared,
    session: &Arc<Session>,
    command: CommandSpec<'_>,
) -> std::io::Result<(ExitStatus, String)> {
    let echo = format!("$ {}", command.display);
    session.log_harness(&format!("[cmd] {}", command.display));
    publish_line(app, shared, &echo);

    let mut child = hidden_command(command.program, command.args).spawn()?;
    let collected = Arc::new(Mutex::new(String::new()));
    if let Some(stdout) = child.stdout.take() {
        spawn_pipe_reader(
            stdout,
            app.clone(),
            session.clone(),
            shared.clone(),
            "[stdout] ",
            Some(collected.clone()),
        );
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_pipe_reader(
            stderr,
            app.clone(),
            session.clone(),
            shared.clone(),
            "[stderr] ",
            None,
        );
    }
    let status = child.wait()?;
    let stdout = collected
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    Ok((status, stdout))
}

fn hidden_command(program: &str, args: &[&str]) -> Command {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

/// 从 `npm list -g` 输出解析本地版本：`(empty)` → None；`@deepseek-ai/dsh@x.y.z` → Some。
fn parse_local_version(out: &str) -> Option<String> {
    if out.contains("(empty)") {
        return None;
    }
    out.split_whitespace()
        .find_map(|tok| tok.strip_prefix("@deepseek-ai/dsh@").map(str::to_string))
}

fn spawn_pipe_reader<R: Read + Send + 'static>(
    reader: R,
    app: AppHandle,
    session: Arc<Session>,
    shared: Shared,
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
            publish_line(&app, &shared, &clean);
            if let Some(collected) = &collect {
                let mut collected = collected.lock().unwrap_or_else(|error| error.into_inner());
                collected.push_str(&clean);
                collected.push('\n');
            }
        }
    });
}

fn publish_line(app: &AppHandle, shared: &SharedRuntime, line: &str) {
    push_output(shared, line);
    let _ = app.emit("harness-line", line);
}

/// 去掉控制字符（保留换行与制表符），避免日志被当作终端/HTML 内容注入。
fn sanitize(line: &str) -> String {
    line.chars()
        .filter(|&c| c == '\n' || c == '\t' || c >= ' ')
        .collect()
}

fn push_output(shared: &SharedRuntime, line: &str) {
    let mut output = shared
        .output
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    output.push_str(line);
    output.push('\n');
    if output.len() > OUTPUT_LIMIT {
        let start = output.len() - OUTPUT_LIMIT;
        let cut = output
            .char_indices()
            .find(|(index, _)| *index >= start)
            .map_or(output.len(), |(index, _)| index);
        *output = output[cut..].to_string();
    }
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
    process::kill_active();
    app.exit(0);
}

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}
