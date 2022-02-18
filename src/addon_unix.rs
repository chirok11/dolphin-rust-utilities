#[napi]
fn kill_process_by_pid(pid: i32) -> i32 {
    debug!("libc::kill({}, libc::SIGINT)", &pid);
    unsafe { libc::kill(pid, libc::SIGINT) }
}