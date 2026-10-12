// capture.rs — Windows GDI 屏幕捕获（主显示器物理像素）+ 结果落盘
use base64::Engine;
use serde_json::{json, Value};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Manager};

pub struct LastCapture {
    pub png_base64: String,
    pub width: u32,
    pub height: u32,
}

pub fn store() -> &'static Mutex<Option<LastCapture>> {
    static STORE: OnceLock<Mutex<Option<LastCapture>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(None))
}

#[cfg(windows)]
pub fn grab_primary() -> Result<LastCapture, String> {
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, GetSystemMetrics, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER,
        CAPTUREBLT, DIB_RGB_COLORS, HDC, SRCCOPY,
    };
    use windows::Win32::UI::WindowsAndMessaging::{SM_CXSCREEN, SM_CYSCREEN};

    unsafe {
        let w = GetSystemMetrics(SM_CXSCREEN);
        let h = GetSystemMetrics(SM_CYSCREEN);
        if w <= 0 || h <= 0 {
            return Err("无法获取屏幕尺寸".into());
        }
        let (w, h) = (w as i32, h as i32);

        let src_dc = GetDC(None);
        if src_dc.is_invalid() {
            return Err("GetDC 失败".into());
        }
        let guard = ScopeGuard::new(|| ReleaseDC(None, src_dc));

        let mem_dc = CreateCompatibleDC(Some(src_dc));
        let bmp = CreateCompatibleBitmap(src_dc, w, h);
        let old = SelectObject(mem_dc, bmp.into());

        let mut result: Result<LastCapture, String> = Err("BitBlt 失败".into());
        if BitBlt(mem_dc, 0, 0, w, h, Some(src_dc), 0, 0, SRCCOPY | CAPTUREBLT)
            .ok()
            .is_ok()
        {
            let mut bmi: BITMAPINFO = std::mem::zeroed();
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = w;
            bmi.bmiHeader.biHeight = -h;
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = 0; // BI_RGB
            let mut buf = vec![0u8; (w as usize) * (h as usize) * 4];
            let got = GetDIBits(
                mem_dc,
                bmp,
                0,
                h as u32,
                buf.as_mut_ptr().cast(),
                &bmi,
                DIB_RGB_COLORS,
            );
            result = if got == h {
                for px in buf.chunks_exact_mut(4) {
                    px.swap(0, 2);
                    px[3] = 255;
                }
                let img =
                    image::RgbaImage::from_raw(w as u32, h as u32, buf).ok_or("缓冲尺寸错误")?;
                let mut png = Vec::new();
                image::DynamicImage::ImageRgba8(img)
                    .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
                    .map_err(|e| format!("PNG 编码失败: {e}"))?;
                Ok(LastCapture {
                    png_base64: base64::engine::general_purpose::STANDARD.encode(png),
                    width: w as u32,
                    height: h as u32,
                })
            } else {
                Err("GetDIBits 失败".into());
            };
        }

        SelectObject(mem_dc, old);
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(mem_dc);
        drop(guard);
        result
    }
}

// RAII 清理 DC
struct ScopeGuard<F: FnOnce()> {
    f: Option<F>,
}
impl<F: FnOnce()> ScopeGuard<F> {
    fn new(f: F) -> Self {
        Self { f: Some(f) }
    }
}
impl<F: FnOnce()> Drop for ScopeGuard<F> {
    fn drop(&mut self) {
        if let Some(f) = self.f.take() {
            f()
        }
    }
}

#[cfg(not(windows))]
pub fn grab_primary() -> Result<LastCapture, String> {
    Err("截屏仅支持 Windows".into())
}

#[tauri::command]
pub fn start_region(app: AppHandle) -> Result<Value, String> {
    let cap = grab_primary()?;
    *store().lock().unwrap() = Some(cap);
    let win = app
        .get_webview_window("overlay")
        .ok_or("overlay 窗口未创建")?;
    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;
    win.eval("window.__onShow && window.__onShow()").ok();
    Ok(json!({"ok": true}))
}

#[tauri::command]
pub fn start_full(app: AppHandle) -> Result<Value, String> {
    let cap = grab_primary()?;
    *store().lock().unwrap() = Some(cap);
    if let Some(win) = app.get_webview_window("editor") {
        let _ = win.emit("editor:load", json!({"full": true}));
        win.show().map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
    } else {
        return Err("editor 窗口未创建".into());
    }
    Ok(json!({"ok": true}))
}

#[tauri::command]
pub fn get_capture() -> Result<Value, String> {
    let guard = store().lock().unwrap();
    match guard.as_ref() {
        Some(c) => Ok(json!({
            "png": c.png_base64,
            "width": c.width,
            "height": c.height,
        })),
        None => Err("还没有截屏".into()),
    }
}

/// 编辑器回传裁剪/合成结果，暂存给主窗「问问 codex」用
#[tauri::command]
pub fn handoff_crop(png_base64: String) -> Result<Value, String> {
    let (w, h) = png_size(&png_base64)?;
    *store().lock().unwrap() = Some(LastCapture {
        png_base64,
        width: w,
        height: h,
    });
    Ok(json!({"ok": true}))
}

fn png_size(png_base64: &str) -> Result<(u32, u32), String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(png_base64)
        .map_err(|_| "base64 无效")?;
    let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
    Ok((img.width(), img.height()))
}

/// 落盘 Pictures/SnipChat + 复制进剪贴板
#[tauri::command]
pub fn save_result(app: AppHandle, png_base64: String) -> Result<Value, String> {
    use std::io::Write;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&png_base64)
        .map_err(|_| "base64 无效")?;

    let dir = dirs::picture_dir()
        .ok_or("找不到图片目录")?
        .join("SnipChat");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let now = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let path = dir.join(format!("截图_{now}.png"));
    std::fs::File::create(&path)
        .and_then(|mut f| f.write_all(&bytes))
        .map_err(|e| e.to_string())?;

    // 剪贴板
    if let Ok(img) = image::load_from_memory(&bytes) {
        let rgba = img.to_rgba8();
        let (w, h) = (rgba.width() as usize, rgba.height() as usize);
        let _ = app.run_on_main_thread(move || {
            if let Ok(mut cb) = arboard::Clipboard::new() {
                let _ = cb.set_image(arboard::ImageData {
                    width: w,
                    height: h,
                    bytes: std::borrow::Cow::Owned(rgba.into_raw()),
                });
            }
        });
    }
    Ok(json!({"ok": true, "path": path.to_string_lossy()}))
}
