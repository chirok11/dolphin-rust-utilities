use kernel32::{CloseHandle, OpenProcess, TerminateProcess};
use winapi::shared::minwindef::LPARAM;
use winapi::shared::windef::HWND;
use winapi::um::winuser::{
  EnumWindows, GetWindowThreadProcessId, SetForegroundWindow, ShowWindow,
};

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_wins(hwnd: *mut winapi::shared::windef::HWND__, l: LPARAM) -> i32 {
  let z = window_thread_process_id(hwnd);
  if z.0 == l as u32 {
    SetForegroundWindow(hwnd);
    ShowWindow(hwnd, 1);
    0
  } else {
    1
  }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_wins_close(
  hwnd: *mut winapi::shared::windef::HWND__,
  l: LPARAM,
) -> i32 {
    use winapi::um::winuser::EndTask;

  let z = window_thread_process_id(hwnd);
  if z.0 == l as u32 {
    EndTask(hwnd, 0, 0);
    0
  } else {
    1
  }
}

#[cfg(target_os = "windows")]
pub fn window_thread_process_id(hwnd: HWND) -> (u32, u32) {
  let mut process_id: u32 = 0;
  let thread_id = unsafe { GetWindowThreadProcessId(hwnd, &mut process_id) };

  (process_id, thread_id)
}

#[cfg(target_os = "windows")]
#[napi]
fn kill_process_by_pid(pid: u32) -> i32 {
  let h = unsafe { OpenProcess(1, 0, pid) };
  if h.is_null() {
    return -1;
  }
  let result = unsafe { TerminateProcess(h, 9) };
  unsafe { CloseHandle(h) };
  result
}

#[cfg(target_os = "windows")]
#[napi]
fn close_process_by_pid(pid: u32) -> i32 {
  unsafe { EnumWindows(Some(enum_wins_close), pid.try_into().unwrap()) }
}

#[cfg(target_os = "windows")]
#[napi]
fn set_foreground_by_pid(pid: u32) -> i32 {
  unsafe { EnumWindows(Some(enum_wins), pid.try_into().unwrap()) }
}

#[test]
fn test_close_process_by_pid() {
  let pid = 11836;
  let r = close_process_by_pid(pid);
  println!("res: {}", r);
}