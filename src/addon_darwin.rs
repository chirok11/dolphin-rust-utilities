#[cfg(target_os = "macos")]
#[napi]
unsafe fn set_foreground_by_pid(pid: u32) -> bool {
    debug!("[macos]: setting foreground by pid: {}", &pid);
    use cocoa::base::id;
    use objc::{class, msg_send, sel, sel_impl};

    let p: id = msg_send![
        class!(NSRunningApplication),
        runningApplicationWithProcessIdentifier: pid
    ];
    match p.is_null() {
        true => false,
        false => {
            debug!("[macos]: activateWithOptions: 2");
            let _: id = msg_send![p, activateWithOptions: 2];
            true
        }
    }
}