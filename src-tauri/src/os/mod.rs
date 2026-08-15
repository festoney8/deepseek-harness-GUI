//! 平台相关能力的统一出口。
//!
//! Windows 与 Unix（macOS/Linux）的差异实现收敛在本目录：
//! 进程树管理、命令构造、PATH 修复。其余模块只依赖本目录导出的统一接口，
//! 不出现任何平台分支。

#[cfg(target_os = "windows")]
mod windows;
#[cfg(unix)]
mod unix;

#[cfg(target_os = "windows")]
pub use windows::*;
#[cfg(unix)]
pub use unix::*;