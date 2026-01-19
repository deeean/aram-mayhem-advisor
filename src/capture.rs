use image::{DynamicImage, RgbaImage};
use windows::core::w;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
    GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    RGBQUAD, SRCCOPY,
};
use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetClientRect, GetDesktopWindow, GetForegroundWindow, GetSystemMetrics,
    GetWindowThreadProcessId, GetWindowRect, SM_CXSCREEN, SM_CYSCREEN,
};

pub struct ScreenSize {
    pub width: i32,
    pub height: i32,
}

pub fn get_screen_size() -> ScreenSize {
    unsafe {
        ScreenSize {
            width: GetSystemMetrics(SM_CXSCREEN),
            height: GetSystemMetrics(SM_CYSCREEN),
        }
    }
}

fn create_bitmap_info(width: i32, height: i32) -> BITMAPINFO {
    let mut bmi: BITMAPINFOHEADER = unsafe { std::mem::zeroed() };

    bmi.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.biWidth = width;
    bmi.biHeight = -height; // top-down DIB
    bmi.biPlanes = 1;
    bmi.biBitCount = 32;
    bmi.biCompression = BI_RGB.0;
    bmi.biSizeImage = 0;
    bmi.biXPelsPerMeter = 0;
    bmi.biYPelsPerMeter = 0;
    bmi.biClrUsed = 0;
    bmi.biClrImportant = 0;

    BITMAPINFO {
        bmiHeader: bmi,
        bmiColors: [RGBQUAD::default(); 1],
    }
}

pub fn capture_screen() -> Option<DynamicImage> {
    let size = get_screen_size();
    capture_region(0, 0, size.width, size.height)
}

pub fn capture_region(x: i32, y: i32, width: i32, height: i32) -> Option<DynamicImage> {
    unsafe {
        let hwnd: HWND = GetDesktopWindow();
        let h_window_dc = GetDC(Some(hwnd));

        let h_dc = CreateCompatibleDC(Some(h_window_dc));
        if h_dc.is_invalid() {
            let _ = ReleaseDC(Some(hwnd), h_window_dc);
            return None;
        }

        let h_bitmap = CreateCompatibleBitmap(h_window_dc, width, height);
        if h_bitmap.is_invalid() {
            let _ = DeleteDC(h_dc);
            let _ = ReleaseDC(Some(hwnd), h_window_dc);
            return None;
        }

        let old_obj = SelectObject(h_dc, h_bitmap.into());
        if old_obj.is_invalid() {
            let _ = DeleteObject(h_bitmap.into());
            let _ = DeleteDC(h_dc);
            let _ = ReleaseDC(Some(hwnd), h_window_dc);
            return None;
        }

        if BitBlt(h_dc, 0, 0, width, height, Some(h_window_dc), x, y, SRCCOPY).is_err() {
            let _ = SelectObject(h_dc, old_obj);
            let _ = DeleteObject(h_bitmap.into());
            let _ = DeleteDC(h_dc);
            let _ = ReleaseDC(Some(hwnd), h_window_dc);
            return None;
        }

        let mut bitmap_info = create_bitmap_info(width, height);
        let size = (width * height) as usize * 4;
        let mut buf: Vec<u8> = vec![0; size];

        GetDIBits(
            h_dc,
            h_bitmap,
            0,
            height as u32,
            Some(buf.as_mut_ptr() as *mut _),
            &mut bitmap_info,
            DIB_RGB_COLORS,
        );

        let _ = SelectObject(h_dc, old_obj);
        let _ = DeleteObject(h_bitmap.into());
        let _ = DeleteDC(h_dc);
        let _ = ReleaseDC(Some(hwnd), h_window_dc);

        for i in (0..buf.len()).step_by(4) {
            buf.swap(i, i + 2);
        }

        RgbaImage::from_raw(width as u32, height as u32, buf)
            .map(DynamicImage::ImageRgba8)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GameWindow {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

pub fn get_lol_window() -> Option<GameWindow> {
    unsafe {
        let hwnd = FindWindowW(w!("RiotWindowClass"), None).ok()?;
        if hwnd.is_invalid() {
            return None;
        }

        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect).ok()?;

        let mut client_rect = RECT::default();
        GetClientRect(hwnd, &mut client_rect).ok()?;

        Some(GameWindow {
            x: rect.left,
            y: rect.top + (rect.bottom - rect.top - client_rect.bottom),
            width: client_rect.right,
            height: client_rect.bottom,
        })
    }
}

pub fn is_lol_foreground() -> bool {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return false;
        }

        let mut process_id: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));

        if process_id == 0 {
            return false;
        }

        let process_handle = OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            false,
            process_id,
        );

        let Ok(handle) = process_handle else {
            return false;
        };

        let mut buffer = [0u16; 260];
        let len = GetModuleFileNameExW(Some(handle), None, &mut buffer);

        if len == 0 {
            return false;
        }

        let path = String::from_utf16_lossy(&buffer[..len as usize]);
        let file_name = path.rsplit('\\').next().unwrap_or("");
        file_name.eq_ignore_ascii_case("League of Legends.exe")
    }
}
