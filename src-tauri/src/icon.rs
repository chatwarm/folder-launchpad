//! Windows 图标提取：从 .lnk/.exe 提取文件关联图标，编码为 base64 PNG data URL。
//!
//! 流程：SHGetFileInfoW 取 HICON -> DrawIconEx 绘制到 32bit DIB -> GetDIBits 取像素
//! -> 手写 PNG 编码（zlib via flate2）-> base64 data URL。

/// 从文件路径提取图标，返回 "data:image/png;base64,..." 字符串；失败返回 None。
/// 非 Windows 平台直接返回 None（前端回退 emoji）。
#[cfg(target_os = "windows")]
pub fn extract_icon_png(path: &str) -> Option<String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Graphics::Gdi::DeleteObject;
    use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
    use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};

    const SIZE: i32 = 48;

    let wide: Vec<u16> = OsStr::new(path).encode_wide().chain(std::iter::once(0)).collect();
    let mut info: SHFILEINFOW = unsafe { std::mem::zeroed() };
    let ret = unsafe {
        SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    };
    if ret == 0 {
        let err = unsafe { windows::Win32::Foundation::GetLastError() };
        crate::debug_log(&format!("icon: SHGetFileInfoW fail err={:?} path={}", err, path));
        return None;
    }

    let icon = info.hIcon;
    if icon.0 == 0 {
        crate::debug_log(&format!("icon: hIcon invalid path={}", path));
        return None;
    }
    let result = hicon_to_png(icon, SIZE, path);

    unsafe {
        let _ = DeleteObject(windows::Win32::Graphics::Gdi::HGDIOBJ(icon.0));
    }
    result
}

#[cfg(not(target_os = "windows"))]
pub fn extract_icon_png(_path: &str) -> Option<String> {
    None
}

#[cfg(target_os = "windows")]
fn hicon_to_png(
    icon: windows::Win32::UI::WindowsAndMessaging::HICON,
    size: i32,
    path: &str,
) -> Option<String> {
    use std::ptr;
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDIBits,
        GetDC, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };
    use windows::Win32::UI::WindowsAndMessaging::{DrawIconEx, DI_NORMAL};

    let hdc_screen = unsafe { GetDC(None) };
    if hdc_screen.is_invalid() {
        crate::debug_log(&format!("icon: GetDC fail path={}", path));
        return None;
    }
    let mem_dc = unsafe { CreateCompatibleDC(hdc_screen) };
    if mem_dc.is_invalid() {
        crate::debug_log(&format!("icon: CreateCompatibleDC fail path={}", path));
        unsafe { let _ = ReleaseDC(None, hdc_screen); }
        return None;
    }

    let mut bmi: BITMAPINFO = unsafe { std::mem::zeroed() };
    bmi.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: size,
        biHeight: -size, // 负值 = 自上而下
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        biSizeImage: 0,
        biXPelsPerMeter: 0,
        biYPelsPerMeter: 0,
        biClrUsed: 0,
        biClrImportant: 0,
    };

    let mut bits: *mut core::ffi::c_void = ptr::null_mut();
    let hbmp = match unsafe {
        CreateDIBSection(mem_dc, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
    } {
        Ok(h) => h,
        Err(e) => {
            crate::debug_log(&format!("icon: CreateDIBSection fail {:?} path={}", e, path));
            unsafe {
                let _ = DeleteDC(mem_dc);
                let _ = ReleaseDC(None, hdc_screen);
            }
            return None;
        }
    };

    let old = unsafe { SelectObject(mem_dc, hbmp) };
    let drawn = unsafe {
        DrawIconEx(mem_dc, 0, 0, icon, size, size, 0, None, DI_NORMAL)
    }.is_ok();

    let png = if !drawn {
        crate::debug_log(&format!("icon: DrawIconEx fail path={}", path));
        None
    } else {
        let byte_len = (size * size * 4) as usize;
        let mut bgra = vec![0u8; byte_len];
        unsafe {
            let _ = GetDIBits(
                mem_dc,
                hbmp,
                0,
                size as u32,
                Some(bgra.as_mut_ptr() as *mut core::ffi::c_void),
                &mut bmi,
                DIB_RGB_COLORS,
            );
        }
        let mut rgba = vec![0u8; byte_len];
        for i in (0..byte_len).step_by(4) {
            rgba[i] = bgra[i + 2];
            rgba[i + 1] = bgra[i + 1];
            rgba[i + 2] = bgra[i];
            rgba[i + 3] = bgra[i + 3];
        }
        match encode_png(&rgba, size as u32, size as u32) {
            Ok(v) => Some(v),
            Err(_) => {
                crate::debug_log(&format!("icon: png encode fail path={}", path));
                None
            }
        }
    };

    unsafe {
        let _ = SelectObject(mem_dc, old);
        let _ = DeleteObject(hbmp);
        let _ = DeleteDC(mem_dc);
        let _ = ReleaseDC(None, hdc_screen);
    }

    png
}

/// 将 RGBA 像素编码为 base64 PNG data URL
#[cfg(target_os = "windows")]
fn encode_png(rgba: &[u8], width: u32, height: u32) -> Result<String, ()> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    fn chunk(tag: &[u8], data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(tag);
        out.extend_from_slice(data);
        let crc = crc32(tag, data);
        out.extend_from_slice(&crc.to_be_bytes());
        out
    }

    let mut buf: Vec<u8> = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    buf.extend_from_slice(&chunk(b"IHDR", &ihdr));

    let stride = (width * 4) as usize;
    let mut raw = Vec::with_capacity((stride + 1) * height as usize);
    for y in 0..height as usize {
        raw.push(0);
        raw.extend_from_slice(&rgba[y * stride..(y + 1) * stride]);
    }
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::fast());
    enc.write_all(&raw).map_err(|_| ())?;
    let idat = enc.finish().map_err(|_| ())?;
    buf.extend_from_slice(&chunk(b"IDAT", &idat));

    buf.extend_from_slice(&chunk(b"IEND", &[]));

    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&buf);
    Ok(format!("data:image/png;base64,{}", b64))
}

#[cfg(target_os = "windows")]
fn crc32(tag: &[u8], data: &[u8]) -> u32 {
    let mut crc: u32 = 0xffffffff;
    for &b in tag.iter().chain(data.iter()) {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb88320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}
