use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, Sender};
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
pub const PORT_END: u16 = 5090;
const START_TIMEOUT: Duration = Duration::from_secs(120);
const POLL_INTERVAL: Duration = Duration::from_millis(500);
const LAST_LOG_LIMIT: usize = 2048;

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    CheckingNode,
    FindingPort,
    Starting,
    Ready,
    Failed,
    EnvMissing,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub phase: Phase,
    pub port: Option<u16>,
    pub detail: String,
    pub elapsed: Option<u64>,
    pub last_log: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Intent {
    Retry,
    Cancel,
}

/// 前端意图入口：worker 持有接收端，命令侧持有发送端。
pub struct Supervisor {
    tx: Sender<Intent>,
    rx: Mutex<Option<Receiver<Intent>>>,
    snap: Arc<Mutex<Snapshot>>,
    session_dir: Arc<Mutex<Option<std::path::PathBuf>>>,
}

impl Supervisor {
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            tx,
            rx: Mutex::new(Some(rx)),
            snap: Arc::new(Mutex::new(Snapshot {
                phase: Phase::CheckingNode,
                port: None,
                detail: "正在检测 Node.js 环境…".into(),
                elapsed: None,
                last_log: String::new(),
            })),
            session_dir: Arc::new(Mutex::new(None)),
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        self.snap.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn retry(&self) {
        let _ = self.tx.send(Intent::Retry);
    }

    pub fn cancel(&self) {
        let _ = self.tx.send(Intent::Cancel);
    }

    pub fn session_dir(&self) -> Option<std::path::PathBuf> {
        self.session_dir
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn spawn_worker(&self, app: AppHandle, base: std::path::PathBuf) {
        let rx = self
            .rx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
            .expect("worker already spawned");
        let snap = self.snap.clone();
        let session_dir = self.session_dir.clone();
        std::thread::Builder::new()
            .name("supervisor".into())
            .spawn(move || worker_loop(app, rx, snap, session_dir, base))
            .expect("spawn supervisor thread");
    }
}

fn emit_state(
    app: &AppHandle,
    snap: &Arc<Mutex<Snapshot>>,
    tail: &Arc<Mutex<String>>,
    phase: Phase,
    port: Option<u16>,
    detail: &str,
    elapsed: Option<u64>,
) {
    let s = Snapshot {
        phase,
        port,
        detail: detail.to_string(),
        elapsed,
        last_log: tail.lock().unwrap_or_else(|e| e.into_inner()).clone(),
    };
    *snap.lock().unwrap_or_else(|e| e.into_inner()) = s.clone();
    let _ = app.emit("runtime-state", s);
}

/// 阻塞等待用户意图；返回 false 表示通道已断开（应用退出中）。
fn wait_intent(rx: &Receiver<Intent>) -> bool {
    loop {
        match rx.recv() {
            Ok(Intent::Retry) => return true,
            Ok(Intent::Cancel) => {}
            Err(_) => return false,
        }
    }
}

fn worker_loop(
    app: AppHandle,
    rx: Receiver<Intent>,
    snap: Arc<Mutex<Snapshot>>,
    session_dir: Arc<Mutex<Option<std::path::PathBuf>>>,
    base: std::path::PathBuf,
) {
    let tail = Arc::new(Mutex::new(String::new()));
    loop {
        // 每次尝试重置最近输出
        *tail.lock().unwrap_or_else(|e| e.into_inner()) = String::new();
        let _ = logs::cleanup_old(&base, Duration::from_secs(14 * 86_400));

        // 1. 环境检测
        emit_state(
            &app,
            &snap,
            &tail,
            Phase::CheckingNode,
            None,
            "正在检测 Node.js 环境…",
            None,
        );
        let env = check_env();
        if !env.ok {
            emit_state(
                &app,
                &snap,
                &tail,
                Phase::EnvMissing,
                None,
                &env.detail,
                None,
            );
            if !wait_intent(&rx) {
                return;
            }
            continue;
        }

        // 2. 端口查找
        emit_state(
            &app,
            &snap,
            &tail,
            Phase::FindingPort,
            None,
            "正在检查端口占用…",
            None,
        );
        let Some(port) = find_port() else {
            emit_state(
                &app,
                &snap,
                &tail,
                Phase::Failed,
                None,
                "端口 3080-5090 均被占用，请检查占用程序后重试。",
                None,
            );
            if !wait_intent(&rx) {
                return;
            }
            continue;
        };

        // 3. 日志会话（创建失败阻断启动）
        let Some(session) = create_session(&base, &session_dir) else {
            emit_state(
                &app,
                &snap,
                &tail,
                Phase::Failed,
                Some(port),
                "无法创建日志目录，已停止启动。请检查磁盘空间或权限后重试。",
                None,
            );
            if !wait_intent(&rx) {
                return;
            }
            continue;
        };

        // 4. 启动 harness
        let mut child = match spawn_harness(port, &base) {
            Ok((child, job)) => {
                set_job(job);
                child
            }
            Err(e) => {
                emit_state(
                    &app,
                    &snap,
                    &tail,
                    Phase::Failed,
                    Some(port),
                    &format!("启动失败：{e}"),
                    None,
                );
                if !wait_intent(&rx) {
                    return;
                }
                continue;
            }
        };
        emit_state(
            &app,
            &snap,
            &tail,
            Phase::Starting,
            Some(port),
            "正在启动 DeepSeek Harness…",
            Some(0),
        );

        if let Some(out) = child.stdout.take() {
            spawn_pipe_reader(out, session.clone(), tail.clone(), "[stdout] ");
        }
        if let Some(err) = child.stderr.take() {
            spawn_pipe_reader(err, session.clone(), tail.clone(), "[stderr] ");
        }

        // 5. 就绪轮询（进程存活 + HTTP 可响应，120 秒超时，可取消）
        let started = Instant::now();
        let ready = 'poll: {
            loop {
                if let Ok(Intent::Cancel) = rx.try_recv() {
                    kill_job();
                    emit_state(
                        &app,
                        &snap,
                        &tail,
                        Phase::Failed,
                        Some(port),
                        "启动已取消。",
                        None,
                    );
                    break 'poll false;
                }
                match child.try_wait() {
                    Ok(Some(status)) => {
                        kill_job();
                        emit_state(
                            &app,
                            &snap,
                            &tail,
                            Phase::Failed,
                            Some(port),
                            &format!("进程提前退出（退出码 {:?}），请查看日志。", status.code()),
                            None,
                        );
                        break 'poll false;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        emit_state(
                            &app,
                            &snap,
                            &tail,
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
                    kill_job();
                    emit_state(
                        &app,
                        &snap,
                        &tail,
                        Phase::Failed,
                        Some(port),
                        "启动超时（120 秒），请查看日志。",
                        None,
                    );
                    break 'poll false;
                }
                emit_state(
                    &app,
                    &snap,
                    &tail,
                    Phase::Starting,
                    Some(port),
                    "正在启动 DeepSeek Harness…",
                    Some(started.elapsed().as_secs()),
                );
                std::thread::sleep(POLL_INTERVAL);
            }
        };
        if !ready {
            if !wait_intent(&rx) {
                return;
            }
            continue;
        }

        // 6. Ready：监控进程，异常退出恢复窗口
        emit_state(
            &app,
            &snap,
            &tail,
            Phase::Ready,
            Some(port),
            "服务已就绪",
            None,
        );
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
                        &tail,
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
        if !wait_intent(&rx) {
            return;
        }
    }
}

fn create_session(
    base: &Path,
    session_dir: &Arc<Mutex<Option<std::path::PathBuf>>>,
) -> Option<Arc<Session>> {
    let session = Arc::new(Session::create(base, std::process::id()).ok()?);
    *session_dir.lock().unwrap_or_else(|e| e.into_inner()) = Some(session.dir.clone());
    Some(session)
}

fn spawn_pipe_reader<R: Read + Send + 'static>(
    reader: R,
    session: Arc<Session>,
    tail: Arc<Mutex<String>>,
    prefix: &'static str,
) {
    std::thread::spawn(move || {
        let lines = BufReader::new(reader).lines();
        for line in lines.map_while(Result::ok) {
            let clean = sanitize(&line);
            session.log_harness(&format!("{prefix}{clean}"));
            if !clean.is_empty() {
                push_tail(&tail, &clean);
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

fn push_tail(tail: &Mutex<String>, line: &str) {
    let mut t = tail.lock().unwrap_or_else(|e| e.into_inner());
    t.push_str(line);
    t.push('\n');
    if t.len() > LAST_LOG_LIMIT {
        let start = t.len() - LAST_LOG_LIMIT;
        let cut = t
            .char_indices()
            .find(|(i, _)| *i >= start)
            .map(|(i, _)| i)
            .unwrap_or(t.len());
        *t = t[cut..].to_string();
    }
}

struct EnvReport {
    ok: bool,
    detail: String,
}

/// npx 是 .cmd 脚本，CreateProcess 无法直接解析，必须经 cmd /C 执行。
fn run_hidden(prog: &str, args: &[&str]) -> Option<String> {
    let mut cmd = Command::new(prog);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn check_env() -> EnvReport {
    let node = run_hidden("node", &["--version"]);
    let npx = run_hidden("cmd", &["/C", "npx", "--version"]);
    let detail = match (&node, &npx) {
        (Some(_), Some(_)) => String::new(),
        (None, _) => "未检测到 Node.js，请从官网（nodejs.org）安装后点击重试。".into(),
        (_, None) => "未检测到 npx，请确认 npm 安装完整后重试。".into(),
    };
    EnvReport {
        ok: node.is_some() && npx.is_some(),
        detail,
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

/// dsh 实际 CLI 契约（经 --help 验证）：dsh --profile web [--host 127.0.0.1] [--port N]
fn spawn_harness(port: u16, work_dir: &Path) -> std::io::Result<(Child, HarnessJob)> {
    let job = HarnessJob::create()?;
    let port = port.to_string();
    let mut cmd = Command::new("cmd");
    cmd.args([
        "/C",
        "npx",
        "--yes",
        "@deepseek-ai/dsh",
        "--profile",
        "web",
        "--host",
        "127.0.0.1",
        "--port",
        port.as_str(),
    ])
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
