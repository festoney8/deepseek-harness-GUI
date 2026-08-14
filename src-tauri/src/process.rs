use std::io;
use std::path::Path;
use std::process::{Child, ChildStderr, ChildStdout, Command, Stdio};
use std::sync::{Mutex, OnceLock};

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

pub struct Harness {
    child: Child,
}

impl Harness {
    pub fn spawn(port: u16, work_dir: &Path) -> io::Result<Self> {
        let job = HarnessJob::create()?;
        let port = port.to_string();
        let mut command = Command::new("cmd");
        command
            .args([
                "/C",
                "dsh",
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
        command.creation_flags(CREATE_NO_WINDOW);

        let mut child = command.spawn()?;
        if let Err(error) = job.assign(&child) {
            let _ = child.kill();
            return Err(error);
        }
        replace_job(job);
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

pub fn kill_active() {
    if let Some(job) = take_job() {
        job.kill();
    }
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

#[cfg(windows)]
struct HarnessJob(HANDLE);

#[cfg(windows)]
// SAFETY: The handle is moved into the process-wide mutex and accessed by one owner at a time.
unsafe impl Send for HarnessJob {}

#[cfg(windows)]
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

#[cfg(windows)]
impl Drop for HarnessJob {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

#[cfg(not(windows))]
struct HarnessJob;

#[cfg(not(windows))]
impl HarnessJob {
    fn create() -> io::Result<Self> {
        Ok(Self)
    }

    fn assign(&self, _child: &Child) -> io::Result<()> {
        Ok(())
    }

    fn kill(&self) {}
}
