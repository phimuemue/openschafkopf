use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;
use std::fs::metadata;
use winapi::{
    um::winuser::{FindWindowA, IsWindow},
};
use injrs::{
    process_windows::Process,
    inject_windows::InjectorExt,
};

fn main() {
    let mut cmd_netschafkopf = [
        "C:\\Program Files\\Cutesoft\\NetSchafkopf\\NetSchk.exe",
        "C:\\Program Files (x86)\\Cutesoft\\NetSchafkopf\\NetSchk.exe",
    ].into_iter()
        .find_map(|str_path| {
            Command::new(str_path)
                .stdout(Stdio::null()) // TODO
                .stderr(Stdio::null()) // TODO
                .spawn()
                .ok()
        })
        .unwrap();
    println!("NetSchafkopf started. Waiting until window exists...");
    loop {
        let str_netschafkopf = std::ffi::CString::new("NetSchafkopf").unwrap();
        let hwnd = unsafe {FindWindowA(
            str_netschafkopf.as_ptr(),
            std::ptr::null(),
        )};
        if 
            hwnd != std::ptr::null_mut()
            && unsafe { IsWindow(hwnd) } != 0
        {
            println!("Window found. Injecting DLL...");
            break;
        }
        println!("Window not yet found. Retrying...");
        sleep(Duration::from_secs(1)); // Sleep briefly before the next check
    }
    let str_dll = [
        "target/i686-pc-windows-gnu/debug/netschafkopf_helper.dll",
        "target/i686-pc-windows-gnu/release/netschafkopf_helper.dll",
    ].iter()
        .filter_map(|str_dll| metadata(str_dll)
            .and_then(|meta| meta.modified().map(|systime| (str_dll, systime)))
            .ok()
        )
        .max_by_key(|&(_str_dll, systime_modified)| systime_modified)
        .unwrap()
        .0;
    println!("Injecting {str_dll}...");
    Process::from_pid(cmd_netschafkopf.id())
        .unwrap()
        .inject(str_dll)
        .unwrap();
    println!("Injected {str_dll}.");
    let exit_status = cmd_netschafkopf.wait().unwrap();
    println!("External executable exited with {:?}", exit_status);
}
