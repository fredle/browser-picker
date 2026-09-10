//! Multi-monitor placement: centre on whichever monitor the cursor is on.

use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

struct WorkArea {
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    /// Physical pixels per logical point on that monitor.
    scale: f32,
}

fn cursor_monitor_workarea() -> Option<WorkArea> {
    unsafe {
        let mut pt = POINT::default();
        GetCursorPos(&mut pt).ok()?;
        let hmon = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);

        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(hmon, &mut mi).as_bool() {
            return None;
        }

        let mut dpi_x = 0u32;
        let mut dpi_y = 0u32;
        let scale = match GetDpiForMonitor(hmon, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) {
            Ok(()) if dpi_x > 0 => dpi_x as f32 / 96.0,
            _ => 1.0,
        };

        let r = mi.rcWork;
        Some(WorkArea {
            left: r.left as f32,
            top: r.top as f32,
            width: (r.right - r.left) as f32,
            height: (r.bottom - r.top) as f32,
            scale,
        })
    }
}

/// Top-left position, in logical points, that centres a `w` x `h` logical-point
/// window on the cursor's monitor. The Win32 work area is in physical pixels, so
/// it has to be divided back out by the monitor's scale factor.
pub fn centered_pos(w: f32, h: f32) -> Option<[f32; 2]> {
    let a = cursor_monitor_workarea()?;
    let x = a.left + (a.width - w * a.scale) / 2.0;
    let y = a.top + (a.height - h * a.scale) / 2.0;
    Some([x / a.scale, y / a.scale])
}
