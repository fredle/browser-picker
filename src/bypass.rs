//! "Hold a key to ignore my rules and let me choose."
//!
//! Read at launch, before any rule is consulted. The modifier is sampled with
//! `GetAsyncKeyState`, which reports the *current* physical key state, so it
//! reflects whether the key is still held at the moment Windows starts us.

use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_SHIFT};

/// Shown in the UI and the README. Change both this and `held()` together.
pub const KEY_NAME: &str = "Shift";

/// Is the bypass modifier held right now?
pub fn held() -> bool {
    // The high bit means "down"; the low bit is a since-last-call toggle we
    // deliberately ignore.
    unsafe { (GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0 }
}
