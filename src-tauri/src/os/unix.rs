//! Unix 平台实现（macOS/Linux 共享）：进程组管理、直接命令调用。

use std::io;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, ChildStderr, ChildStdout, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use libc::{kill, setpgid, SIGKILL, SIGTERM};

/// 两段式杀灭的宽限期：先 SIGTERM 优雅退出，超时后 SIGKILL 兜底。
const TERM_GRACE: Duration = Duration::from_millis(500);

/// dsh harness 子进程句柄，进程树按进程组管理。
pub struct Harness {
    child: Child,
}

impl Harness {
    /// 以 `dsh --profile web --host 127.0.0.1 --port` 启动 harness，置于独立进程组。
    pub fn spawn(port: u16, work_dir: &Path) -> io::Result<Self> {
        let port = port.to_string();
        let mut command = build_command(
            "dsh",
            &["--profile", "web", "--host", "127.0.0.1", "--port", port.as_str()],
        );
        command.current_dir(work_dir);
        // SAFETY: pre_exec 在子进程 exec 前运行，仅调用 async-signal-safe 的 setpgid，
        // 使子进程一出生即处于新进程组（后代全部继承），保证 kill 可整组命中。
        unsafe {
            command.pre_exec(|| {
                setpgid(0, 0);
                Ok(())
            });
        }
        let mut child = command.spawn()?;
        let pgid = child.id() as i32;
        replace_active(ActiveHarness { pgid });
        log::debug!("harness spawned (pid {})", child.id());
        Ok(Self { child })
    }

    pub fn stdout(&mut self) -> Option<ChildStdout> {
        self.child.stdout.take()
    }

    pub fn stderr(&mut self) -> Option<ChildStderr> {
        self.child.stderr.take()
    }

    pub fn try_wait(&mut self) -> io::Result<Option<std::process::ExitStatus>> {
        self.child.try_wait()
    }
}

/// 终止当前 active harness 的进程组：SIGTERM 后留宽限期，再 SIGKILL 兜底。
pub fn kill_active() {
    if let Some(active) = take_active() {
        log::debug!("killing active harness process group");
        active.kill();
    }
}

/// 构造命令：Unix 直接调用程序本体，IO 接管道。
pub fn build_command(program: &str, args: &[&str]) -> Command {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

/// 修复 PATH：读取 shell 配置，保证 GUI 启动的进程能找到 brew/nvm 安装的命令。
pub fn fix_env_path() {
    let _ = fix_path_env::fix();
}

struct ActiveHarness {
    pgid: i32,
}

impl ActiveHarness {
    fn kill(&self) {
        unsafe {
            kill(-self.pgid, SIGTERM);
        }
        std::thread::sleep(TERM_GRACE);
        unsafe {
            kill(-self.pgid, SIGKILL);
        }
    }
}

static ACTIVE: OnceLock<Mutex<Option<ActiveHarness>>> = OnceLock::new();

fn active() -> &'static Mutex<Option<ActiveHarness>> {
    ACTIVE.get_or_init(|| Mutex::new(None))
}

fn take_active() -> Option<ActiveHarness> {
    active()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .take()
}

fn replace_active(harness: ActiveHarness) {
    if let Some(previous) = take_active() {
        previous.kill();
    }
    *active()
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(harness);
}