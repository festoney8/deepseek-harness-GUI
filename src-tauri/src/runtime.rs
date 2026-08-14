use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
#[cfg(windows)]
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
#[cfg(windows)]
use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

use crate::logs::{self, Session};

pub const PORT_START: u16 = 3080;
pub const PORT_END: u16 = 5080;
const START_TIMEOUT: Duration = Duration::from_secs(120);
const POLL_INTERVAL: Duration = Duration::from_millis(500);
/// UI 输出缓冲上限，超出丢弃最旧保留最新
const OUTPUT_LIMIT: usize = 1_048_576;

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Idle,
    Installing,
    Starting,
    Ready,
    Failed,
}

/// 三格面板 + 终端的完整状态
#[derive(Clone, Serialize)]
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
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Intent {
    CheckEnv,
    CheckVersion,
    Install,
    Start,
    Cancel,
}

/// 前端意图入口：worker 持有接收端，命令侧持有发送端。
pub struct Supervisor {
    tx: std::sync::mpsc::Sender<Intent>,
    rx: Mutex<Option<Receiver<Intent>>>,
    snap: Arc<Mutex<Snapshot>>,
    session_dir: Arc<Mutex<Option<std::path::PathBuf>>>,
    output: Arc<Mutex<String>>,
}

impl Supervisor {
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            tx,
            rx: Mutex::new(Some(rx)),
            snap: Arc::new(Mutex::new(Snapshot {
                phase: Phase::Idle,
                port: None,
                detail: String::new(),
                elapsed: None,
                node: None,
                npm: None,
                remote: None,
                local: None,
                version_error: false,
            })),
            session_dir: Arc::new(Mutex::new(None)),
            output: Arc::new(Mutex::new(String::new())),
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        self.snap.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// 完整命令行输出缓冲（stdout+stderr 合并）
    pub fn output(&self) -> String {
        self.output.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn session_dir(&self) -> Option<std::path::PathBuf> {
        self.session_dir.lock().unwrap_or_else(|e| e.into_inner()).clone()
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

    pub fn cancel(&self) {
        self.send(Intent::Cancel);
    }

    pub fn spawn_worker(&self, app: AppHandle, base: std::path::PathBuf) {
        let rx = self.rx.lock().unwrap_or_else(|e| e.into_inner()).take().expect("worker already spawned");
        let snap = self.snap.clone();
        let session_dir = self.session_dir.clone();
        let output = self.output.clone();
        std::thread::Builder::new()
            .name("supervisor".into())
            .spawn(move || worker_loop(app, rx, snap, session_dir, output, base))
            .expect("spawn supervisor thread");
    }
}

fn emit_state(
    app: &AppHandle,
    snap: &Arc<Mutex<Snapshot>>,
    phase: Phase,
    port: Option<u16>,
    detail: &str,
    elapsed: Option<u64>,
    update: impl FnOnce(&mut Snapshot),
) {
    let mut s = snap.lock().unwrap_or_else(|e| e.into_inner());
    update(&mut s);
    s.phase = phase;
    s.port = port;
    s.detail = detail.to_string();
    s.elapsed = elapsed;
    let clone = s.clone();
    drop(s);
    let _ = app.emit("runtime-state", clone);
}

fn worker_loop(
    app: AppHandle,
    rx: Receiver<Intent>,
    snap: Arc<Mutex<Snapshot>>,
    session_dir: Arc<Mutex<Option<std::path::PathBuf>>>,
    output: Arc<Mutex<String>>,
    base: std::path::PathBuf,
) {
    *output.lock().unwrap_or_else(|e| e.into_inner()) = String::new();
    let _ = app.emit("harness-output-reset", ());
    let _ = logs::cleanup_old(&base, Duration::from_secs(14 * 86_400));

    let Some(session) = create_session(&base, &session_dir) else {
        emit_state(&app, &snap, Phase::Failed, None, "无法创建日志目录，请检查磁盘空间或权限。", None, |_| {});
        return;
    };

    // 启动即自动检查环境与版本，填充格子
    check_env(&app, &snap, &session, &output);
    check_version(&app, &snap, &session, &output);
    emit_state(&app, &snap, Phase::Idle, None, "就绪", None, |s| {
        s.port = find_port();
    });

    loop {
        match rx.recv() {
            Err(_) => return,
            Ok(Intent::CheckEnv) => {
                check_env(&app, &snap, &session, &output);
                emit_state(&app, &snap, Phase::Idle, None, "就绪", None, |s| {
                    s.port = find_port();
                });
            }
            Ok(Intent::CheckVersion) => {
                check_version(&app, &snap, &session, &output);
                emit_state(&app, &snap, Phase::Idle, None, "就绪", None, |s| {
                    s.port = find_port();
                });
            }
            Ok(Intent::Install) => {
                emit_state(&app, &snap, Phase::Installing, None, "正在安装/更新 DeepSeek Harness…", None, |_| {});
                let ok = match run_cmd_streamed(
                    &app,
                    &output,
                    &session,
                    "npm i -g @deepseek-ai/dsh",
                    "cmd",
                    &["/C", "npm", "i", "-g", "@deepseek-ai/dsh"],
                ) {
                    Ok((status, _)) => status.success(),
                    Err(_) => false,
                };
                if ok {
                    check_version(&app, &snap, &session, &output);
                    emit_state(&app, &snap, Phase::Idle, None, "就绪", None, |s| {
                        s.port = find_port();
                    });
                } else {
                    emit_state(&app, &snap, Phase::Failed, None, "安装失败，请查看终端输出。", None, |_| {});
                }
            }
            Ok(Intent::Start) => {
                let Some(port) = find_port() else {
                    emit_state(&app, &snap, Phase::Failed, None, "端口 3080-5080 均被占用，请检查占用程序。", None, |_| {});
                    continue;
                };
                let mut child = match spawn_harness(port, &base) {
                    Ok((child, job)) => {
                        set_job(job);
                        child
                    }
                    Err(e) => {
                        emit_state(&app, &snap, Phase::Failed, Some(port), &format!("启动失败：{e}"), None, |_| {});
                        continue;
                    }
                };
                emit_state(&app, &snap, Phase::Starting, Some(port), "正在启动 DeepSeek Harness…", Some(0), |_| {});

                if let Some(out) = child.stdout.take() {
                    spawn_pipe_reader(out, app.clone(), session.clone(), output.clone(), "[stdout] ", None);
                }
                if let Some(err) = child.stderr.take() {
                    spawn_pipe_reader(err, app.clone(), session.clone(), output.clone(), "[stderr] ", None);
                }

                // 就绪轮询（进程存活 + HTTP 可响应，120 秒超时，可取消）
                let started = Instant::now();
                let ready = 'poll: {
                    loop {
                        if let Ok(Intent::Cancel) = rx.try_recv() {
                            kill_job();
                            emit_state(&app, &snap, Phase::Failed, Some(port), "启动已取消。", None, |_| {});
                            break 'poll false;
                        }
                        match child.try_wait() {
                            Ok(Some(status)) => {
                                kill_job();
                                emit_state(
                                    &app,
                                    &snap,
                                    Phase::Failed,
                                    Some(port),
                                    &format!("进程提前退出（退出码 {:?}），请查看终端输出。", status.code()),
                                    None,
                                    |_| {},
                                );
                                break 'poll false;
                            }
                            Ok(None) => {}
                            Err(e) => {
                                emit_state(&app, &snap, Phase::Failed, Some(port), &format!("无法读取进程状态：{e}"), None, |_| {});
                                break 'poll false;
                            }
                        }
                        if http_ok(port) {
                            break 'poll true;
                        }
                        if started.elapsed() > START_TIMEOUT {
                            kill_job();
                            emit_state(&app, &snap, Phase::Failed, Some(port), "启动超时（120 秒），请查看终端输出。", None, |_| {});
                            break 'poll false;
                        }
                        emit_state(
                            &app,
                            &snap,
                            Phase::Starting,
                            Some(port),
                            "正在启动 DeepSeek Harness…",
                            Some(started.elapsed().as_secs()),
                            |_| {},
                        );
                        std::thread::sleep(POLL_INTERVAL);
                    }
                };
                if !ready {
                    continue;
                }

                // Ready：监控进程，异常退出恢复窗口
                emit_state(&app, &snap, Phase::Ready, Some(port), "服务已就绪", None, |_| {});
                session.log_gui(&format!("harness ready on http://127.0.0.1:{port}"));
                loop {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            let code = status.code();
                            kill_job();
                            if code == Some(0) {
                                session.log_gui("harness exited with code 0, quitting");
                                shutdown(&app);
                                return;
                            }
                            session.log_gui(&format!("harness exited with code {code:?}"));
                            emit_state(
                                &app,
                                &snap,
                                Phase::Failed,
                                Some(port),
                                &format!("DeepSeek Harness 已退出（退出码 {code:?}）。"),
                                None,
                                |_| {},
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
            Ok(Intent::Cancel) => {
                kill_job();
                emit_state(&app, &snap, Phase::Failed, None, "已取消。", None, |_| {});
            }
        }
    }
}

/// 检查 node / npm 版本，写入格子一
fn check_env(app: &AppHandle, snap: &Arc<Mutex<Snapshot>>, session: &Arc<Session>, output: &Arc<Mutex<String>>) {
    let node = stream_capture(app, output, session, "node -v", "node", &["-v"]);
    let npm = stream_capture(app, output, session, "npm -v", "cmd", &["/C", "npm", "-v"]);
    let mut s = snap.lock().unwrap_or_else(|e| e.into_inner());
    s.node = node;
    s.npm = npm;
}

/// 检查远端 / 本地 dsh 版本，写入格子二
fn check_version(app: &AppHandle, snap: &Arc<Mutex<Snapshot>>, session: &Arc<Session>, output: &Arc<Mutex<String>>) {
    let remote = stream_capture(app, output, session, "npm view @deepseek-ai/dsh version", "cmd", &["/C", "npm", "view", "@deepseek-ai/dsh", "version"]);
    let local_out = stream_capture(app, output, session, "npm list -g @deepseek-ai/dsh version", "cmd", &["/C", "npm", "list", "-g", "@deepseek-ai/dsh", "version"]).unwrap_or_default();
    let local = parse_local_version(&local_out);
    let mut s = snap.lock().unwrap_or_else(|e| e.into_inner());
    s.remote = remote.clone();
    s.local = local;
    s.version_error = remote.is_none();
}

/// 执行一次性命令并捕获 stdout（成功时返回 trim 后的全文）
fn stream_capture(
    app: &AppHandle,
    output: &Arc<Mutex<String>>,
    session: &Arc<Session>,
    cmdline: &str,
    prog: &str,
    args: &[&str],
) -> Option<String> {
    match run_cmd_streamed(app, output, session, cmdline, prog, args) {
        Ok((status, out)) if status.success() => Some(out.trim().to_string()),
        _ => None,
    }
}

fn create_session(base: &Path, session_dir: &Arc<Mutex<Option<std::path::PathBuf>>>) -> Option<Arc<Session>> {
    let session = Arc::new(Session::create(base, std::process::id()).ok()?);
    *session_dir.lock().unwrap_or_else(|e| e.into_inner()) = Some(session.dir.clone());
    Some(session)
}

/// 执行一次性命令：向终端回显 `$ <cmdline>`，stdout/stderr 实时写入
/// harness.log、输出缓冲并 emit "harness-line"；返回退出码与 stdout 全文。
fn run_cmd_streamed(
    app: &AppHandle,
    output: &Arc<Mutex<String>>,
    session: &Arc<Session>,
    cmdline: &str,
    prog: &str,
    args: &[&str],
) -> std::io::Result<(ExitStatus, String)> {
    let echo = format!("$ {cmdline}");
    session.log_harness(&format!("[cmd] {cmdline}"));
    push_output(output, &echo);
    let _ = app.emit("harness-line", echo);

    let mut cmd = Command::new(prog);
    cmd.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let mut child = cmd.spawn()?;

    let collected = Arc::new(Mutex::new(String::new()));
    if let Some(out) = child.stdout.take() {
        spawn_pipe_reader(out, app.clone(), session.clone(), output.clone(), "[stdout] ", Some(collected.clone()));
    }
    if let Some(err) = child.stderr.take() {
        spawn_pipe_reader(err, app.clone(), session.clone(), output.clone(), "[stderr] ", None);
    }
    let status = child.wait()?;
    let stdout_full = collected.lock().unwrap_or_else(|e| e.into_inner()).clone();
    Ok((status, stdout_full))
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
    output: Arc<Mutex<String>>,
    prefix: &'static str,
    collect: Option<Arc<Mutex<String>>>,
) {
    std::thread::spawn(move || {
        let lines = BufReader::new(reader).lines();
        for line in lines.map_while(Result::ok) {
            let clean = sanitize(&line);
            session.log_harness(&format!("{prefix}{clean}"));
            if !clean.is_empty() {
                push_output(&output, &clean);
                let _ = app.emit("harness-line", &clean);
                if let Some(c) = &collect {
                    c.lock().unwrap_or_else(|e| e.into_inner()).push_str(&clean);
                    c.lock().unwrap_or_else(|e| e.into_inner()).push('\n');
                }
            }
        }
    });
}

/// 去掉控制字符（保留换行与制表符），避免日志被当作终端/HTML 内容注入。
fn sanitize(line: &str) -> String {
    line.chars().filter(|&c| c == '\n' || c == '\t' || c >= ' ').collect()
}

fn push_output(output: &Mutex<String>, line: &str) {
    let mut o = output.lock().unwrap_or_else(|e| e.into_inner());
    o.push_str(line);
    o.push('\n');
    if o.len() > OUTPUT_LIMIT {
        let start = o.len() - OUTPUT_LIMIT;
        let cut = o.char_indices().find(|(i, _)| *i >= start).map(|(i, _)| i).unwrap_or(o.len());
        *o = o[cut..].to_string();
    }
}

fn port_free(port: u16) -> bool {
    TcpStream::connect_timeout(&SocketAddr::from(([127, 0, 0, 1], port)), Duration::from_millis(200)).is_err()
}

fn find_port() -> Option<u16> {
    (PORT_START..=PORT_END).find(|&p| port_free(p))
}

/// 就绪探测：GET / 读到 HTTP 状态行即认为服务可响应。
fn http_ok(port: u16) -> bool {
    let Ok(mut s) = TcpStream::connect_timeout(&SocketAddr::from(([127, 0, 0, 1], port)), Duration::from_millis(500)) else {
        return false;
    };
    let _ = s.set_read_timeout(Some(Duration::from_millis(500)));
    if s.write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n").is_err() {
        return false;
    }
    let mut buf = [0u8; 64];
    let Ok(n) = s.read(&mut buf) else {
        return false;
    };
    String::from_utf8_lossy(&buf[..n]).starts_with("HTTP/")
}

/// dsh 为全局安装的 .cmd 入口，CreateProcess 无法直接解析，必须经 cmd /C。
/// CLI 契约（经 --help 验证）：dsh --profile web [--host 127.0.0.1] [--port N]
fn spawn_harness(port: u16, work_dir: &Path) -> std::io::Result<(Child, HarnessJob)> {
    let job = HarnessJob::create()?;
    let port = port.to_string();
    let mut cmd = Command::new("cmd");
    cmd.args(["/C", "dsh", "--profile", "web", "--host", "127.0.0.1", "--port", port.as_str()])
        .current_dir(work_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let child = cmd.spawn()?;
    job.assign(&child)?;
    Ok((child, job))
}

/// Windows Job Object：整个进程树随句柄回收被终止，退出路径唯一且幂等。
#[cfg(windows)]
pub struct HarnessJob(HANDLE);

// Job 句柄是内核对象引用，跨线程共享安全
#[cfg(windows)]
unsafe impl Send for HarnessJob {}
#[cfg(windows)]
unsafe impl Sync for HarnessJob {}

#[cfg(windows)]
impl HarnessJob {
    pub fn create() -> std::io::Result<Self> {
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err(std::io::Error::last_os_error());
            }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let ok = SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const core::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            if ok == 0 {
                CloseHandle(handle);
                return Err(std::io::Error::last_os_error());
            }
            Ok(Self(handle))
        }
    }

    pub fn assign(&self, child: &Child) -> std::io::Result<()> {
        unsafe {
            if AssignProcessToJobObject(self.0, child.as_raw_handle() as HANDLE) == 0 {
                return Err(std::io::Error::last_os_error());
            }
        }
        Ok(())
    }

    pub fn kill(&self) {
        unsafe {
            TerminateJobObject(self.0, 1);
        }
    }
}

#[cfg(windows)]
impl Drop for HarnessJob {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

#[cfg(not(windows))]
pub struct HarnessJob;

#[cfg(not(windows))]
impl HarnessJob {
    pub fn create() -> std::io::Result<Self> {
        Ok(Self)
    }
    pub fn assign(&self, _child: &Child) -> std::io::Result<()> {
        Ok(())
    }
    pub fn kill(&self) {}
}

static JOB_SLOT: OnceLock<Mutex<Option<HarnessJob>>> = OnceLock::new();

fn job_slot() -> &'static Mutex<Option<HarnessJob>> {
    JOB_SLOT.get_or_init(|| Mutex::new(None))
}

fn take_job() -> Option<HarnessJob> {
    job_slot().lock().unwrap_or_else(|e| e.into_inner()).take()
}

fn set_job(job: HarnessJob) {
    if let Some(prev) = take_job() {
        prev.kill();
    }
    *job_slot().lock().unwrap_or_else(|e| e.into_inner()) = Some(job);
}

fn kill_job() {
    if let Some(job) = take_job() {
        job.kill();
    }
}

/// 唯一关闭入口：先终止 harness 进程树，再退出应用。
/// KILL_ON_JOB_CLOSE 兜底：任何遗漏路径下句柄随进程关闭，OS 回收整个进程树。
pub fn shutdown(app: &AppHandle) {
    kill_job();
    app.exit(0);
}

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}