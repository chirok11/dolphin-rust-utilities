#[napi]
fn kill_process_by_pid(pid: i32) -> i32 {
    unsafe { libc::kill(pid, libc::SIGINT) }
}