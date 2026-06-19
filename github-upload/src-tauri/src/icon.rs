// Windows 图标提取：从 .lnk/.url/.exe/文件夹等提取 HICON，转为 PNG 缓存到磁盘。
// 仅依赖 crate::lnk 与已配置的 windows/image/blake3/rust_ini。
#![allow(dead_code)]

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder, RgbaImage};
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DeleteDC, DeleteObject,
    DIB_RGB_COLORS, GetDIBits, GetObjectW, SelectObject, HBITMAP, HDC, HGDIOBJ,
};
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
use windows::Win32::UI::Controls::IImageList;
use windows::Win32::UI::Shell::{
    SHGetFileInfoW, SHGetImageList, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON,
    SHGFI_SYSICONINDEX, SHGFI_USEFILEATTRIBUTES,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, GetIconInfo, LoadIconW, PrivateExtractIconsW, HICON, ICONINFO, IDI_APPLICATION,
};

use crate::lnk;

const SHIL_EXTRALARGE: i32 = 2;
const SHIL_JUMBO: i32 = 4;

/// 把 UTF-8 字符串转为以 0 结尾的 UTF-16 wide string。
fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

/// 把路径填充为 PrivateExtractIconsW 要求的定长 260 (MAX_PATH) 宽字符数组。
/// 超长（去掉结尾 0 后 > 259）则返回 None。
fn to_wide_260(s: &str) -> Option<[u16; 260]> {
    let mut buf = [0u16; 260];
    let mut iter = OsStr::new(s).encode_wide();
    for slot in buf.iter_mut().take(259) {
        match iter.next() {
            Some(ch) => *slot = ch,
            None => return Some(buf),
        }
    }
    // 检查是否还有剩余字符未写入（说明过长）。
    if iter.next().is_some() {
        None
    } else {
        Some(buf)
    }
}

/// 为给定路径提取/缓存图标 PNG。命中缓存直接返回路径。
/// 修复 Bug 2：提取链路尽力返回一个有效图标（几乎不返回 None）。
pub fn get_icon_url(path: &str, cache_dir: &Path) -> Option<PathBuf> {
    // 1. 计算缓存路径：cache_dir/launchpad_icons/<blake3 hex>.png
    let key = blake3::hash(path.as_bytes()).to_hex();
    let icons_dir = cache_dir.join("launchpad_icons");
    let cache_path = icons_dir.join(format!("{}.png", key));

    if cache_path.exists() {
        return Some(cache_path);
    }

    // 2. 解析最终用于提取的“逻辑路径”（.lnk/.url 可能指向别处）
    let resolved = resolve_target_path(path);

    // 3. 提取 HICON：主链路（jumbo/extralarge/largeicon/private）
    let hicon = extract_hicon(&resolved)
        .or_else(|| extract_hicon(path))
        // Bug 2 兜底 1：用目标文件本身拿关联图标（不带 USEFILEATTRIBUTES，按真实路径）
        .or_else(|| unsafe { extract_associated_icon(&resolved) })
        .or_else(|| unsafe { extract_associated_icon(path) })
        // Bug 2 兜底 2：通用程序图标
        .or_else(|| load_default_application_icon());

    // 即使 HICON 拿到，hicon_to_png 仍可能失败；失败再尝试通用图标。
    let png = match hicon {
        Some(h) => match hicon_to_png(h) {
            Some(bytes) => bytes,
            None => {
                // 再拿一个通用图标做最后转换尝试。
                let fallback = load_default_application_icon()?;
                hicon_to_png(fallback)?
            }
        },
        None => return None,
    };

    // 5. 写入缓存
    if std::fs::create_dir_all(&icons_dir).is_ok() {
        let _ = std::fs::write(&cache_path, &png);
    }

    Some(cache_path)
}

/// 对 .lnk/.url：优先返回其声明的图标来源路径；否则返回原路径。
/// 其他扩展名直接返回原路径。
fn resolve_target_path(path: &str) -> String {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".lnk") {
        if let Ok(info) = lnk::parse_lnk(path) {
            // 优先 target（exe，能拿到大图标）；icon_location 可能是小图标或失效，作备选
            if let Some(target) = info.target.as_deref() {
                if !target.is_empty() {
                    return target.to_string();
                }
            }
            if let Some(icon) = info.icon_location.as_deref() {
                if !icon.is_empty() {
                    return normalize(icon, Path::new(path).parent());
                }
            }
        }
        return path.to_string();
    }
    if lower.ends_with(".url") {
        if let Some(icon_file) = parse_url_icon_file(path) {
            return normalize(&icon_file, Path::new(path).parent());
        }
        return path.to_string();
    }
    path.to_string()
}

/// 把可能是相对路径的 icon 归一化（相对 .lnk/.url 父目录解析）。
fn normalize(icon: &str, parent: Option<&Path>) -> String {
    let p = Path::new(icon);
    if p.is_absolute() {
        return icon.to_string();
    }
    // 去掉可能的 ",index" 后缀（部分 IconFile 写成 "C:\x.ico,0"）
    let icon = icon.rsplit_once(',').map(|(base, _)| base).unwrap_or(icon);
    let p = Path::new(icon);
    if p.is_absolute() {
        return icon.to_string();
    }
    if let Some(parent) = parent {
        if let Ok(joined) = dunce::canonicalize(parent.join(p)) {
            return joined.to_string_lossy().into_owned();
        }
        return parent.join(p).to_string_lossy().into_owned();
    }
    icon.to_string()
}

/// 解析 .url 的 [InternetShortcut] IconFile。
fn parse_url_icon_file(url_path: &str) -> Option<String> {
    let content = std::fs::read_to_string(url_path).ok()?;
    let ini = ini::Ini::load_from_str(&content).ok()?;
    let section = ini.section(Some("InternetShortcut"))?;
    let icon_file = section.get("IconFile")?;
    let trimmed = icon_file.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// 提取 HICON 的多级回退：
/// 1) SHGetFileInfoW(SHGFI_SYSICONINDEX) → SHGetImageList(SHIL_JUMBO) → GetIcon
/// 2) SHIL_EXTRALARGE
/// 3) SHGetFileInfoW(SHGFI_ICON | SHGFI_LARGEICON) 直接拿 hIcon
/// 4) Bug 1 修复：对 .exe 用 PrivateExtractIcons 按 256→128→64→48→32 取最大可用图标
fn extract_hicon(path: &str) -> Option<HICON> {
    lnk::ensure_com();
    unsafe {
        // ---- 1) jumbo via image list ----
        if let Some(h) = extract_via_image_list(path, SHIL_JUMBO) {
            return Some(h);
        }
        // ---- 2) PrivateExtractIcons（仅对 .exe，优先取 256 等大图标）----
        if is_exe_path(path) {
            if let Some(h) = extract_via_private(path) {
                return Some(h);
            }
        }
        // ---- 3) extralarge (48) ----
        if let Some(h) = extract_via_image_list(path, SHIL_EXTRALARGE) {
            return Some(h);
        }
        // ---- 4) 兜底：SHGFI_LARGEICON 关联图标 (32) ----
        if let Some(h) = extract_shgfi_largeicon(path) {
            return Some(h);
        }
    }
    None
}

fn is_exe_path(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(OsStr::to_str)
        .map(|e| e.eq_ignore_ascii_case("exe"))
        .unwrap_or(false)
}

/// SHGetFileInfoW(SHGFI_ICON | SHGFI_LARGEICON | SHGFI_USEFILEATTRIBUTES) 直接拿 hIcon。
unsafe fn extract_shgfi_largeicon(path: &str) -> Option<HICON> {
    let path_w = to_wide(path);
    let mut shfi = SHFILEINFOW::default();
    let cb = std::mem::size_of::<SHFILEINFOW>() as u32;
    let flags = SHGFI_ICON | SHGFI_LARGEICON | SHGFI_USEFILEATTRIBUTES;
    let _ret = SHGetFileInfoW(
        PCWSTR(path_w.as_ptr()),
        FILE_ATTRIBUTE_NORMAL,
        Some(&mut shfi as *mut _),
        cb,
        flags,
    );
    if !shfi.hIcon.is_invalid() {
        Some(shfi.hIcon)
    } else {
        None
    }
}

/// Bug 2 兜底：用文件真实路径（不使用 USEFILEATTRIBUTES）拿文件关联图标。
unsafe fn extract_associated_icon(path: &str) -> Option<HICON> {
    let path_w = to_wide(path);
    let mut shfi = SHFILEINFOW::default();
    let cb = std::mem::size_of::<SHFILEINFOW>() as u32;
    let flags = SHGFI_ICON | SHGFI_LARGEICON;
    let _ret = SHGetFileInfoW(
        PCWSTR(path_w.as_ptr()),
        FILE_ATTRIBUTE_NORMAL,
        Some(&mut shfi as *mut _),
        cb,
        flags,
    );
    if !shfi.hIcon.is_invalid() {
        Some(shfi.hIcon)
    } else {
        None
    }
}

/// Bug 2 最终兜底：通用程序图标 IDI_APPLICATION。
fn load_default_application_icon() -> Option<HICON> {
    lnk::ensure_com();
    unsafe { LoadIconW(None, IDI_APPLICATION).ok() }
}

/// 通过 SHGetImageList 取得指定尺寸的图标。失败返回 None（且不泄漏）。
unsafe fn extract_via_image_list(path: &str, shil: i32) -> Option<HICON> {
    let path_w = to_wide(path);
    let mut shfi = SHFILEINFOW::default();
    let cb = std::mem::size_of::<SHFILEINFOW>() as u32;
    let flags = SHGFI_SYSICONINDEX | SHGFI_USEFILEATTRIBUTES;
    let _ret = SHGetFileInfoW(
        PCWSTR(path_w.as_ptr()),
        FILE_ATTRIBUTE_NORMAL,
        Some(&mut shfi as *mut _),
        cb,
        flags,
    );
    let index = shfi.iIcon;
    // 注意：iIcon==0 通常是“默认/未知”图标，但对很多真实文件也会回 0，
    // 因此不做过滤，统一尝试取图标。
    // SHGetImageList 失败很常见（jumbo 在某些环境/主题下不可用）——静默回退。
    let list: IImageList = match SHGetImageList(shil) {
        Ok(l) => l,
        Err(_) => return None,
    };
    match list.GetIcon(index, 0) {
        Ok(h) if !h.is_invalid() => Some(h),
        Ok(h) => {
            // 拿到但无效，仍释放（理论上 GetIcon 成功就有效，稳妥起见）
            let _ = DestroyIcon(h);
            None
        }
        Err(_) => None,
    }
}

/// Bug 1 修复：用 PrivateExtractIconsW 按从大到小尺寸逐个尝试，
/// 取第一个成功提取到的 HICON。flags 用 LR_DEFAULTSIZE（让系统按 cx/cy 缩放）。
unsafe fn extract_via_private(path: &str) -> Option<HICON> {
    let fname = to_wide_260(path)?;
    // 尺寸从大到小，优先取最大可用图标（256→128→64→48→32）。
    for &size in &[256i32, 128, 64, 48, 32] {
        let mut hicon_out: [HICON; 1] = [HICON(std::ptr::null_mut())];
        let mut icon_id: u32 = 0;
        let got = PrivateExtractIconsW(
            &fname,
            0, // nIconIndex
            size,
            size,
            Some(&mut hicon_out),
            Some(&mut icon_id as *mut u32),
            0, // 不用 LR_DEFAULTSIZE（它会强制系统默认尺寸、忽略传入 size）；让 size 生效
        );
        // 返回值 = 实际提取的图标数；为 0xFFFFFFFF(-1) 表示失败。
        if got != 0 && got != u32::MAX {
            let h = hicon_out[0];
            if !h.is_invalid() {
                return Some(h);
            }
        } else if got == u32::MAX {
            // 失败：释放可能被写入的句柄（防御性），继续下个尺寸。
            if !hicon_out[0].is_invalid() {
                let _ = DestroyIcon(hicon_out[0]);
            }
        }
    }
    None
}

/// HICON -> PNG 字节。负责释放所有 GDI 资源（hicon/hbmColor/hbmMask/hdc）。
/// 用 GdiGuard（RAII）保证任何返回路径（含 ? 提前返回）都不泄漏句柄。
///
/// Bug 3 修复：alpha 处理重写——
/// - 先检测整张图的 alpha 是否几乎全 0；
/// - 全 0 → 老式无 alpha 图标，用 hbmMask 1bpp 单色 mask 合成 alpha（镂空变透明）；
/// - 有有效 alpha → 原样保留，绝不补 255。
fn hicon_to_png(hicon: HICON) -> Option<Vec<u8>> {
    unsafe {
        // 拿 ICONINFO；GetIconInfo 失败时仍需手动释放 hicon。
        let mut iconinfo: ICONINFO = std::mem::zeroed();
        if GetIconInfo(hicon, &mut iconinfo as *mut _).is_err() {
            let _ = DestroyIcon(hicon);
            return None;
        }

        // 从此刻起，hicon + 两个 bitmap 都要被释放；交给 guard 统一管理。
        // DC 还没创建，先置 None；成功创建后再赋值。
        let mut guard = GdiGuard {
            hicon,
            hbm_color: iconinfo.hbmColor,
            hbm_mask: iconinfo.hbmMask,
            hdc: None,
        };

        let has_color_bitmap = !iconinfo.hbmColor.is_invalid();
        // 选定要读取的颜色位图：优先 hbmColor；为空（纯单色图标）退到 hbmMask。
        let hbm_color = if has_color_bitmap {
            iconinfo.hbmColor
        } else {
            iconinfo.hbmMask
        };

        // 取尺寸。
        let mut bm: BITMAP = std::mem::zeroed();
        let n = GetObjectW(
            HGDIOBJ::from(hbm_color),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bm as *mut _ as *mut _),
        );
        if n == 0 || bm.bmWidth <= 0 || bm.bmHeight <= 0 {
            return None;
        }
        let w = bm.bmWidth as u32;
        let h = bm.bmHeight as u32;

        // 创建兼容 DC。
        let hdc = CreateCompatibleDC(None);
        if hdc.is_invalid() {
            return None;
        }
        guard.hdc = Some(hdc);

        // 选入位图（取回的旧对象在 DC 释放前无需还原，DC 立即释放即可）。
        let _old = SelectObject(hdc, HGDIOBJ::from(hbm_color));

        // 构造 BITMAPINFO：biHeight 取负 → 自上而下扫描，省去后续翻转。
        let mut bi: BITMAPINFO = std::mem::zeroed();
        bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bi.bmiHeader.biWidth = w as i32;
        bi.bmiHeader.biHeight = -(h as i32);
        bi.bmiHeader.biPlanes = 1;
        bi.bmiHeader.biBitCount = 32;
        bi.bmiHeader.biCompression = BI_RGB.0;

        let mut pixels = vec![0u8; (w as usize) * (h as usize) * 4];
        let got = GetDIBits(
            hdc,
            hbm_color,
            0,
            h as u32,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bi as *mut _,
            DIB_RGB_COLORS,
        );
        if got == 0 {
            return None;
        }

        // ===== Bug 3：alpha 处理 =====
        // 先 BGRA→RGB（保留 alpha 原值待判定）。
        for chunk in pixels.chunks_exact_mut(4) {
            let (b, g, r, a) = (chunk[0], chunk[1], chunk[2], chunk[3]);
            chunk[0] = r;
            chunk[1] = g;
            chunk[2] = b;
            chunk[3] = a; // 暂存原 alpha
        }

        // 统计 alpha 通道：是否有“足够多”的非 0 像素（认为存在有效 alpha 通道）。
        let total = pixels.len() / 4;
        let non_zero_alpha = pixels.chunks_exact(4).filter(|c| c[3] != 0).count();
        let alpha_valid = non_zero_alpha > (total / 32); // > ~3% 视为有 alpha 通道

        if !alpha_valid {
            // 老式无 alpha 图标：用 hbmMask（1bpp）合成 alpha，镂空处变透明。
            // 仅在存在独立 color bitmap（即 hbmMask 是真正的掩码）时才合成；
            // 纯单色图标本身没有“镂空”概念，保持 alpha=255 即可。
            if has_color_bitmap && !iconinfo.hbmMask.is_invalid() {
                if let Some(mask_alpha) = read_mask_alpha(hdc, iconinfo.hbmMask, w, h) {
                    for (chunk, ma) in pixels.chunks_exact_mut(4).zip(mask_alpha.iter()) {
                        chunk[3] = *ma;
                    }
                } else {
                    // mask 读取失败：保守按完全不透明处理（避免全透明黑块）。
                    for chunk in pixels.chunks_exact_mut(4) {
                        chunk[3] = 255;
                    }
                }
            } else {
                // 无独立 mask（纯单色图标）：保持完全不透明。
                for chunk in pixels.chunks_exact_mut(4) {
                    chunk[3] = 255;
                }
            }
        }
        // alpha_valid == true：保留原始 alpha，透明处保持透明（不补 255）。

        // 裁掉四周透明边距，让图标内容填满整张 PNG（通病修复：部分图标内容只占中间小块）。
        let (pixels, w, h) = trim_transparent(pixels, w, h);

        // 编码 PNG。guard 在函数末尾 drop，统一释放 GDI 资源。
        let img = RgbaImage::from_raw(w, h, pixels)?;
        let mut buf = Vec::new();
        PngEncoder::new(&mut buf)
            .write_image(img.as_raw(), w, h, ExtendedColorType::Rgba8)
            .ok()?;
        Some(buf)
    }
}

/// 裁掉 RGBA 图像四周的透明边距，让非透明内容填满整张（通病修复：部分图标内容只占中间小块）。
fn trim_transparent(pixels: Vec<u8>, w: u32, h: u32) -> (Vec<u8>, u32, u32) {
    if w == 0 || h == 0 {
        return (pixels, w, h);
    }
    let mut min_x = w;
    let mut min_y = h;
    let mut max_x: u32 = 0;
    let mut max_y: u32 = 0;
    let mut found = false;
    for y in 0..h {
        for x in 0..w {
            let a = pixels[((y * w + x) * 4 + 3) as usize];
            if a > 8 {
                found = true;
                if x < min_x {
                    min_x = x;
                }
                if y < min_y {
                    min_y = y;
                }
                if x > max_x {
                    max_x = x;
                }
                if y > max_y {
                    max_y = y;
                }
            }
        }
    }
    if !found {
        return (pixels, w, h);
    }
    let nw = max_x - min_x + 1;
    let nh = max_y - min_y + 1;
    let mut out = Vec::with_capacity((nw * nh * 4) as usize);
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let i = ((y * w + x) * 4) as usize;
            out.push(pixels[i]);
            out.push(pixels[i + 1]);
            out.push(pixels[i + 2]);
            out.push(pixels[i + 3]);
        }
    }
    (out, nw, nh)
}

/// 读取 1bpp 单色 mask 位图，生成与 (w*h) 像素一一对应的 alpha 数组。
/// 约定：mask 中像素为“白”(bit=1) → 不透明 255；“黑”(bit=0) → 透明 0。
/// 若读取失败或方向与约定相反，调用方会自行兜底。
unsafe fn read_mask_alpha(hdc: HDC, hbm_mask: HBITMAP, w: u32, h: u32) -> Option<Vec<u8>> {
    // 取 mask 实际尺寸/行字节数（mask 可能比 color 更高——带 AND/XOR 双行结构，
    // 但 32bpp 彩色图标的 mask 通常就是单层 AND mask，高度 == h）。
    let mut bm: BITMAP = std::mem::zeroed();
    let n = GetObjectW(
        HGDIOBJ::from(hbm_mask),
        std::mem::size_of::<BITMAP>() as i32,
        Some(&mut bm as *mut _ as *mut _),
    );
    if n == 0 || bm.bmWidth <= 0 || bm.bmHeight <= 0 {
        return None;
    }

    // 用 GetDIBits 读 1bpp DIB。每行按 4 字节对齐。
    let row_bytes = (((bm.bmWidth as u32) + 31) / 32 * 4) as usize;
    let plane_h = bm.bmHeight as u32;
    let buf_size = row_bytes * plane_h as usize;
    let mut mask_buf = vec![0u8; buf_size];

    let mut bi: BITMAPINFO = std::mem::zeroed();
    bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bi.bmiHeader.biWidth = bm.bmWidth;
    bi.bmiHeader.biHeight = -(plane_h as i32); // 自上而下
    bi.bmiHeader.biPlanes = 1;
    bi.bmiHeader.biBitCount = 1;
    bi.bmiHeader.biCompression = BI_RGB.0;

    let got = GetDIBits(
        hdc,
        hbm_mask,
        0,
        plane_h,
        Some(mask_buf.as_mut_ptr() as *mut _),
        &mut bi as *mut _,
        DIB_RGB_COLORS,
    );
    if got == 0 {
        return None;
    }

    // 把每个像素对应到一个 alpha：bit=1(白) → 255，bit=0(黑) → 0。
    let mut alpha = vec![0u8; (w as usize) * (h as usize)];
    // 仅取 mask 的前 h 行（避免双行结构污染）。
    let rows = (h as usize).min(plane_h as usize);
    for y in 0..rows {
        let row = &mask_buf[y * row_bytes..y * row_bytes + row_bytes];
        for x in 0..w as usize {
            let byte_idx = x / 8;
            let bit_in_byte = 7 - (x % 8); // 1bpp DIB 高位在前
            let byte = row.get(byte_idx).copied().unwrap_or(0);
            let bit = (byte >> bit_in_byte) & 1;
            // 经验方向：bit=1 → 不透明。若实测整体反相，则下面这行翻转即可。
            let a = if bit == 1 { 255 } else { 0 };
            alpha[y * w as usize + x] = a;
        }
    }
    Some(alpha)
}

/// RAII：确保 GDI 句柄在任何返回路径（成功/失败/panic unwind）下都被释放。
struct GdiGuard {
    hicon: HICON,
    hbm_color: HBITMAP,
    hbm_mask: HBITMAP,
    hdc: Option<HDC>,
}

impl Drop for GdiGuard {
    fn drop(&mut self) {
        unsafe {
            if !self.hicon.is_invalid() {
                let _ = DestroyIcon(self.hicon);
            }
            if !self.hbm_color.is_invalid() {
                let _ = DeleteObject(HGDIOBJ::from(self.hbm_color));
            }
            if !self.hbm_mask.is_invalid() {
                let _ = DeleteObject(HGDIOBJ::from(self.hbm_mask));
            }
            if let Some(hdc) = self.hdc {
                if !hdc.is_invalid() {
                    let _ = DeleteDC(hdc);
                }
            }
        }
    }
}
