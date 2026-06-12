#[cfg(target_os = "windows")]
pub fn extract_icon_rgba(exe_path: &str) -> Option<(u32, u32, Vec<u8>)> {
    use windows::Win32::UI::Shell::*;
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::Foundation::*;
    use windows::core::*;

    unsafe {
        let exe_path_wide: Vec<u16> = exe_path.encode_utf16().chain(std::iter::once(0)).collect();

        let mut large_icon = HICON::default();
        let result = ExtractIconExW(
            PCWSTR(exe_path_wide.as_ptr()),
            0,
            Some(&mut large_icon),
            None,
            1,
        );

        if result == 0 || large_icon.is_invalid() {
            return None;
        }

        let mut icon_info = ICONINFO::default();
        if !GetIconInfo(large_icon, &mut icon_info).is_ok() {
            DestroyIcon(large_icon);
            return None;
        }

        let mut bm = BITMAP::default();
        GetObjectW(
            icon_info.hbmColor.into(),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bm as *mut _ as *mut _),
        );

        let width = bm.bmWidth as u32;
        let height = bm.bmHeight as u32;

        let hdc = GetDC(Some(HWND::default()));
        let mem_dc = CreateCompatibleDC(Some(hdc));

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0 as u32,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let dib = CreateDIBSection(
            Some(mem_dc),
            &bmi,
            DIB_RGB_COLORS,
            &mut bits,
            None,
            0,
        ).ok()?;

        let old_bitmap = SelectObject(mem_dc, dib.into());
        DrawIconEx(mem_dc, 0, 0, large_icon, width as i32, height as i32, 0, Some(HBRUSH::default()), DI_NORMAL);

        let pixel_count = (width * height) as usize;
        let src_pixels = std::slice::from_raw_parts(bits as *const u8, pixel_count * 4);
        let mut rgba_data = Vec::with_capacity(pixel_count * 4);

        for i in 0..pixel_count {
            let offset = i * 4;
            let b = src_pixels[offset];
            let g = src_pixels[offset + 1];
            let r = src_pixels[offset + 2];
            let a = src_pixels[offset + 3];

            rgba_data.push(r);
            rgba_data.push(g);
            rgba_data.push(b);
            rgba_data.push(a);
        }

        SelectObject(mem_dc, old_bitmap);
        DeleteObject(dib.into());
        DeleteDC(mem_dc);
        ReleaseDC(Some(HWND::default()), hdc);
        DeleteObject(icon_info.hbmColor.into());
        DeleteObject(icon_info.hbmMask.into());
        DestroyIcon(large_icon);

        Some((width, height, rgba_data))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn extract_icon_rgba(_exe_path: &str) -> Option<(u32, u32, Vec<u8>)> {
    None
}

pub fn extract_icon_to_handle(exe_path: &str) -> Option<iced::widget::image::Handle> {
    extract_icon_rgba(exe_path)
        .map(|(w, h, rgba)| iced::widget::image::Handle::from_rgba(w, h, rgba))
}

/// Nearest-neighbor resize to 64x64, alpha-blend onto the device icon
/// background (0x2124), emit big-endian RGB565 — exactly 8192 bytes.
pub fn rgba_to_rgb565_icon(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    const SIZE: u32 = 64;
    const BG: (u16, u16, u16) = (33, 36, 33);  // device ICON_BG 0x2124
    let mut out = Vec::with_capacity((SIZE * SIZE * 2) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let sx = (x * w / SIZE).min(w - 1);
            let sy = (y * h / SIZE).min(h - 1);
            let i = ((sy * w + sx) * 4) as usize;
            let (r, g, b, a) = (
                rgba[i] as u16, rgba[i + 1] as u16,
                rgba[i + 2] as u16, rgba[i + 3] as u16,
            );
            let r = (r * a + BG.0 * (255 - a)) / 255;
            let g = (g * a + BG.1 * (255 - a)) / 255;
            let b = (b * a + BG.2 * (255 - a)) / 255;
            let px: u16 = ((r >> 3) << 11) | ((g >> 2) << 5) | (b >> 3);
            out.push((px >> 8) as u8);   // high byte first — firmware expects this
            out.push((px & 0xFF) as u8);
        }
    }
    out
}