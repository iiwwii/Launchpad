// .lnk 解析与创建，统一走 COM IShellLinkW（解析/创建 API 一致、确定性强）。
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::core::{Interface, PCWSTR};
use windows::Win32::Storage::FileSystem::WIN32_FIND_DATAW;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, IPersistFile,
    STGM_READ,
};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink, SLGP_RAWPATH};

pub struct LnkInfo {
    pub target: Option<String>,
    pub arguments: Option<String>,
    pub working_dir: Option<String>,
    pub icon_location: Option<String>,
}

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

fn from_wide(buf: &[u16]) -> Option<String> {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    if end == 0 {
        return None;
    }
    let s = String::from_utf16_lossy(&buf[..end]);
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// 确保当前线程进入 COM（MTA）。重复调用幂等；Shell API 在 MTA 下可用。
pub fn ensure_com() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

/// 解析 .lnk 的目标/参数/工作目录/图标位置。
pub fn parse_lnk(path: &str) -> windows::core::Result<LnkInfo> {
    ensure_com();
    unsafe {
        let shell_link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
        let persist_file: IPersistFile = shell_link.cast()?;

        let path_w = to_wide(path);
        persist_file.Load(PCWSTR(path_w.as_ptr()), STGM_READ)?;

        let mut target = [0u16; 260];
        let mut fd = WIN32_FIND_DATAW::default();
        let _ = shell_link.GetPath(&mut target, &mut fd as *mut _, SLGP_RAWPATH.0 as u32);

        let mut args = [0u16; 512];
        let _ = shell_link.GetArguments(&mut args);

        let mut wd = [0u16; 260];
        let _ = shell_link.GetWorkingDirectory(&mut wd);

        let mut icon = [0u16; 260];
        let mut icon_idx = 0i32;
        let _ = shell_link.GetIconLocation(&mut icon, &mut icon_idx as *mut _);

        Ok(LnkInfo {
            target: from_wide(&target),
            arguments: from_wide(&args),
            working_dir: from_wide(&wd),
            icon_location: from_wide(&icon),
        })
    }
}

/// 创建指向 `target` 的 .lnk，写到 `lnk_path`。
pub fn create_lnk(
    target: &str,
    lnk_path: &str,
    args: Option<&str>,
    work_dir: Option<&str>,
    icon: Option<&str>,
) -> windows::core::Result<()> {
    ensure_com();
    unsafe {
        let shell_link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;

        let target_w = to_wide(target);
        shell_link.SetPath(PCWSTR(target_w.as_ptr()))?;

        if let Some(a) = args {
            let w = to_wide(a);
            shell_link.SetArguments(PCWSTR(w.as_ptr()))?;
        }
        if let Some(d) = work_dir {
            let w = to_wide(d);
            shell_link.SetWorkingDirectory(PCWSTR(w.as_ptr()))?;
        }
        if let Some(i) = icon {
            let w = to_wide(i);
            shell_link.SetIconLocation(PCWSTR(w.as_ptr()), 0)?;
        }

        let persist_file: IPersistFile = shell_link.cast()?;
        let lnk_w = to_wide(lnk_path);
        persist_file.Save(PCWSTR(lnk_w.as_ptr()), true)?;

        Ok(())
    }
}
