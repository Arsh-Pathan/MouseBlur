use windows::Win32::UI::WindowsAndMessaging::*;

#[derive(Debug, Clone, Copy)]
pub struct CursorState {
    pub x: i32,
    pub y: i32,
    pub visible: bool,
}

#[inline(always)]
pub fn get_current_cursor_state() -> Option<CursorState> {
    unsafe {
        let mut ci = CURSORINFO {
            cbSize: std::mem::size_of::<CURSORINFO>() as u32,
            ..Default::default()
        };

        if GetCursorInfo(&mut ci).is_err() {
            return None;
        }

        let visible = (ci.flags.0 & CURSOR_SHOWING.0) != 0;

        Some(CursorState {
            x: ci.ptScreenPos.x,
            y: ci.ptScreenPos.y,
            visible,
        })
    }
}
