use std::{
    ffi::c_void,
    io,
    mem::size_of,
    os::windows::{
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
        process::CommandExt,
    },
    process::{Command, Stdio},
    ptr,
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::sync::{Mutex as AsyncMutex, Notify};
use windows_sys::Win32::{
    Foundation::{HANDLE, INVALID_HANDLE_VALUE},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
        },
        JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        },
        Threading::{
            OpenThread, ResumeThread, CREATE_NO_WINDOW, CREATE_SUSPENDED, THREAD_SUSPEND_RESUME,
        },
    },
};

use super::{PlatformError, ProcessExit, ProcessKind, SpawnedProcess};

const JOB_TERMINATION_EXIT_CODE: u32 = 1;

/// Windows 平台上可克隆的受控进程树句柄
#[derive(Debug, Clone)]
pub(crate) struct ManagedProcess {
    inner: Arc<WindowsProcess>,
}

#[derive(Debug)]
struct WindowsProcess {
    kind: ProcessKind,
    job: OwnedJob,
    exit: Mutex<Option<Result<ProcessExit, WaitFailure>>>,
    exited: Notify,
    termination: AsyncMutex<()>,
}

#[derive(Debug)]
struct OwnedJob(OwnedHandle);

impl OwnedJob {
    fn raw(&self) -> HANDLE {
        self.0.as_raw_handle().cast::<c_void>()
    }
}

#[derive(Debug, Clone)]
struct WaitFailure {
    kind: io::ErrorKind,
    message: Arc<str>,
}

impl WaitFailure {
    fn from_io(error: io::Error) -> Self {
        Self {
            kind: error.kind(),
            message: Arc::from(error.to_string()),
        }
    }

    fn to_io(&self) -> io::Error {
        io::Error::new(self.kind, self.message.to_string())
    }
}

impl ManagedProcess {
    /// 非阻塞读取已经缓存的进程退出结果
    pub(crate) fn try_wait(&self) -> Result<Option<ProcessExit>, PlatformError> {
        let exit = self.inner.exit.lock().map_err(|_| PlatformError::Wait {
            kind: self.inner.kind,
            source: io::Error::other("进程退出状态锁已损坏"),
        })?;

        match exit.as_ref() {
            Some(Ok(status)) => Ok(Some(*status)),
            Some(Err(error)) => Err(PlatformError::Wait {
                kind: self.inner.kind,
                source: error.to_io(),
            }),
            None => Ok(None),
        }
    }

    /// 异步等待唯一监督任务缓存进程退出结果
    pub(crate) async fn wait(&self) -> Result<ProcessExit, PlatformError> {
        loop {
            let notified = self.inner.exited.notified();
            if let Some(exit) = self.try_wait()? {
                return Ok(exit);
            }
            notified.await;
        }
    }

    /// 终止 Windows Job Object 中的完整进程树
    pub(crate) async fn terminate_tree(
        &self,
        _grace_period: Duration,
    ) -> Result<ProcessExit, PlatformError> {
        if let Some(exit) = self.try_wait()? {
            return Ok(exit);
        }

        let _termination = self.inner.termination.lock().await;
        if let Some(exit) = self.try_wait()? {
            return Ok(exit);
        }

        terminate_job(self.inner.job.raw(), self.inner.kind)?;
        self.wait().await
    }
}

/// 使用 Windows 命令入口创建受控 DSH 进程树
pub(super) fn spawn_dsh(port: u16) -> Result<SpawnedProcess, PlatformError> {
    spawn_dsh_command("dsh", port)
}

fn spawn_dsh_command(program: &str, port: u16) -> Result<SpawnedProcess, PlatformError> {
    let command = format!("{program} --profile web --port {port} --no-open");
    let mut process = Command::new("cmd.exe");
    process.args(["/D", "/S", "/C", &command]);
    spawn(process, ProcessKind::Dsh)
}

fn spawn(mut command: Command, kind: ProcessKind) -> Result<SpawnedProcess, PlatformError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED);

    let job = create_job(kind)?;
    let mut child = tokio::process::Command::from(command)
        .spawn()
        .map_err(|source| PlatformError::Spawn { kind, source })?;
    let process_id = child.id().ok_or_else(|| PlatformError::Spawn {
        kind,
        source: io::Error::other("子进程没有可用 PID"),
    })?;

    let Some(process_handle) = child.raw_handle() else {
        let _ = child.start_kill();
        return Err(PlatformError::Spawn {
            kind,
            source: io::Error::other("子进程没有可用 Windows handle"),
        });
    };
    if let Err(error) = assign_to_job(job.raw(), process_handle, kind) {
        let _ = child.start_kill();
        return Err(error);
    }
    if let Err(error) = resume_suspended_process(process_id, kind) {
        let _ = child.start_kill();
        return Err(error);
    }

    let stdout = child.stdout.take().ok_or_else(|| PlatformError::Spawn {
        kind,
        source: io::Error::other("子进程 stdout 管道不可用"),
    })?;
    let stderr = child.stderr.take().ok_or_else(|| PlatformError::Spawn {
        kind,
        source: io::Error::other("子进程 stderr 管道不可用"),
    })?;

    let inner = Arc::new(WindowsProcess {
        kind,
        job,
        exit: Mutex::new(None),
        exited: Notify::new(),
        termination: AsyncMutex::new(()),
    });
    tokio::spawn(reap_child(child, Arc::clone(&inner)));

    Ok(SpawnedProcess {
        process: ManagedProcess { inner },
        stdout,
        stderr,
    })
}

async fn reap_child(mut child: tokio::process::Child, process: Arc<WindowsProcess>) {
    let result = child.wait().await.map(|status| ProcessExit {
        exit_code: status.code(),
    });
    let cached = result.map_err(WaitFailure::from_io);

    if let Ok(mut exit) = process.exit.lock() {
        *exit = Some(cached);
    }
    process.exited.notify_waiters();
}

fn create_job(kind: ProcessKind) -> Result<OwnedJob, PlatformError> {
    // SAFETY: Both optional pointers are null, requesting an unnamed Job Object with default security.
    let raw = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
    if raw.is_null() {
        return Err(PlatformError::Spawn {
            kind,
            source: io::Error::last_os_error(),
        });
    }

    // SAFETY: CreateJobObjectW returned an owned handle that is transferred exactly once.
    let job = OwnedJob(unsafe { OwnedHandle::from_raw_handle(raw.cast()) });
    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

    // SAFETY: job remains valid for the call and limits points to a correctly sized initialized value.
    let configured = unsafe {
        SetInformationJobObject(
            job.raw(),
            JobObjectExtendedLimitInformation,
            ptr::from_ref(&limits).cast(),
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    };
    if configured == 0 {
        return Err(PlatformError::Control {
            kind,
            source: io::Error::last_os_error(),
        });
    }

    Ok(job)
}

fn assign_to_job(job: HANDLE, process: HANDLE, kind: ProcessKind) -> Result<(), PlatformError> {
    // SAFETY: Both handles are live for this call; ownership remains with their Rust owners.
    if unsafe { AssignProcessToJobObject(job, process) } == 0 {
        return Err(PlatformError::Control {
            kind,
            source: io::Error::last_os_error(),
        });
    }
    Ok(())
}

fn resume_suspended_process(process_id: u32, kind: ProcessKind) -> Result<(), PlatformError> {
    // SAFETY: The snapshot is read-only and is closed by OwnedHandle after enumeration.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(PlatformError::Control {
            kind,
            source: io::Error::last_os_error(),
        });
    }
    // SAFETY: CreateToolhelp32Snapshot returned an owned handle transferred exactly once.
    let snapshot = unsafe { OwnedHandle::from_raw_handle(snapshot.cast()) };
    let mut entry = THREADENTRY32 {
        dwSize: size_of::<THREADENTRY32>() as u32,
        ..Default::default()
    };
    // SAFETY: entry is a valid writable THREADENTRY32 with its required size initialized.
    let mut has_entry = unsafe { Thread32First(snapshot.as_raw_handle().cast(), &mut entry) };
    let mut resumed = false;

    while has_entry != 0 {
        if entry.th32OwnerProcessID == process_id {
            // SAFETY: The thread ID comes from the live system thread snapshot.
            let thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID) };
            if thread.is_null() {
                return Err(PlatformError::Control {
                    kind,
                    source: io::Error::last_os_error(),
                });
            }
            // SAFETY: thread is an owned handle with suspend/resume access.
            let thread = unsafe { OwnedHandle::from_raw_handle(thread.cast()) };
            // SAFETY: The thread handle is valid and the thread was created suspended.
            if unsafe { ResumeThread(thread.as_raw_handle().cast()) } == u32::MAX {
                return Err(PlatformError::Control {
                    kind,
                    source: io::Error::last_os_error(),
                });
            }
            resumed = true;
        }

        // SAFETY: entry remains valid for the next snapshot record.
        has_entry = unsafe { Thread32Next(snapshot.as_raw_handle().cast(), &mut entry) };
    }

    if resumed {
        Ok(())
    } else {
        Err(PlatformError::Control {
            kind,
            source: io::Error::other("未找到挂起的初始线程"),
        })
    }
}

fn terminate_job(job: HANDLE, kind: ProcessKind) -> Result<(), PlatformError> {
    // SAFETY: job is owned by the shared process state and stays live until all waiters finish.
    if unsafe { TerminateJobObject(job, JOB_TERMINATION_EXIT_CODE) } == 0 {
        let source = io::Error::last_os_error();
        if source.raw_os_error() != Some(6) {
            return Err(PlatformError::Control { kind, source });
        }
    }
    Ok(())
}
