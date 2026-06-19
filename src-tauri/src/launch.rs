// 用 ShellExecuteW 启动/打开任意文件、快捷方式或 URL。
// 不用 std::process::Command（无法处理 .lnk / 关联文件），
// 也不用 tauri-plugin-shell.open（丢 verb=runas）。对 .lnk，Shell 会自动解引用目标。
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::core::PCWSTR;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

pub fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

pub fn launch(path: &str) -> std::io::Result<()> {
    let file = to_wide(path);
    let verb = to_wide("open");
    let hinstance = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    // 出错时返回的 HINSTANCE 值 <= 32
    let code = hinstance.0 as usize;
    if code <= 32 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("ShellExecuteW 失败，错误码 {}", code),
        ));
    }
    Ok(())
}
