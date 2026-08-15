//! Windows 平台实现：Job Object 进程树管理、cmd /C 命令包装、隐藏控制台窗口。

use std::io;
use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Child, ChildStderr, ChildStdout, Command, Stdio};
use std::sync::{Mutex, OnceLock};

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

/// dsh harness 子进程句柄，进程树由 Job Object 托管（句柄关闭即整树回收）。
pub struct Harness {
    child: Child,
}

impl Harness {
    /// 以 `dsh --profile web --host 127.0.0.1 --port` 启动 harness，整树归入 Job Object。
    pub fn spawn(port: u16, work_dir: &Path) -> io::Result<Self> {
        let job = HarnessJob::create()?;
        let port = port.to_string();
        let mut command = build_command(
            "dsh",
            &["--profile", "web", "--host", "127.0.0.1", "--port", port.as_str()],
        );
        command.current_dir(work_dir);

        let mut child = command.spawn()?;
        if let Err(error) = job.assign(&child) {
            log::error!("failed to assign harness to job object: {error}");
            let _ = child.kill();
            return Err(error);
        }
        replace_job(job);
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

/// 终止当前 active harness 的整棵进程树（Job Object 强杀）。
pub fn kill_active() {
    if let Some(job) = take_job() {
        log::debug!("killing active harness job");
        job.kill();
    }
}

/// 构造命令：包装为 `cmd /C`，隐藏控制台窗口，IO 接管道。
pub fn build_command(program: &str, args: &[&str]) -> Command {
    let mut command = Command::new("cmd");
    command
        .arg("/C")
        .arg(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW);
    command
}

/// 修复 PATH：fix-path-env 同时修复 Windows 上 Command 的 PATH 继承问题。
pub fn fix_env_path() {
    let _ = fix_path_env::fix();
}

static ACTIVE_JOB: OnceLock<Mutex<Option<HarnessJob>>> = OnceLock::new();

fn active_job() -> &'static Mutex<Option<HarnessJob>> {
    ACTIVE_JOB.get_or_init(|| Mutex::new(None))
}

fn take_job() -> Option<HarnessJob> {
    active_job()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .take()
}

fn replace_job(job: HarnessJob) {
    if let Some(previous) = take_job() {
        previous.kill();
    }
    *active_job()
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(job);
}

struct HarnessJob(HANDLE);

// SAFETY: 句柄随进程级互斥锁单点访问，可在线程间迁移。
unsafe impl Send for HarnessJob {}

impl HarnessJob {
    fn create() -> io::Result<Self> {
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err(io::Error::last_os_error());
            }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let configured = SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const core::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            if configured == 0 {
                CloseHandle(handle);
                return Err(io::Error::last_os_error());
            }
            Ok(Self(handle))
        }
    }

    fn assign(&self, child: &Child) -> io::Result<()> {
        unsafe {
            if AssignProcessToJobObject(self.0, child.as_raw_handle() as HANDLE) == 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }

    fn kill(&self) {
        unsafe {
            TerminateJobObject(self.0, 1);
        }
    }
}

impl Drop for HarnessJob {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}