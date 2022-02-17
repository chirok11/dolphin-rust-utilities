#[cfg(target_os = "windows")]
use winapi::{um::winuser::{SetForegroundWindow, GetWindowThreadProcessId, EnumWindows, ShowWindow}, shared::{windef::{HWND__, HWND}, minwindef::LPARAM}};

#[cfg(target_os = "windows")]
#[napi]
fn kill_process_by_pid(pid: u32) -> u32 {
    let h = unsafe { OpenProcess(1, 0, pid) };
  let result = unsafe { TerminateProcess(h, 9) };
  unsafe { CloseHandle(h) };
}

#[cfg(target_os = "windows")]
#[napi]
fn set_foreground_by_pid(pid: u32) -> u32 {
  unsafe { EnumWindows(Some(enum_wins), pid) }
}