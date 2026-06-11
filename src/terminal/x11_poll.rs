use crossterm::event::KeyModifiers;
use std::sync::OnceLock;

// X11 Keysym definitions
const XK_SHIFT_L: std::os::raw::c_ulong = 0xFFE1;
const XK_SHIFT_R: std::os::raw::c_ulong = 0xFFE2;
const XK_CONTROL_L: std::os::raw::c_ulong = 0xFFE3;
const XK_CONTROL_R: std::os::raw::c_ulong = 0xFFE4;
const XK_ALT_L: std::os::raw::c_ulong = 0xFFE9;
const XK_ALT_R: std::os::raw::c_ulong = 0xFFEA;

// Unix dynamic loading flags
const RTLD_LAZY: std::os::raw::c_int = 1;

unsafe extern "C" {
    fn dlopen(filename: *const std::os::raw::c_char, flag: std::os::raw::c_int) -> *mut std::ffi::c_void;
    fn dlsym(handle: *mut std::ffi::c_void, symbol: *const std::os::raw::c_char) -> *mut std::ffi::c_void;
}

struct X11Lib {
    x_open_display: unsafe extern "C" fn(*const std::os::raw::c_char) -> *mut std::ffi::c_void,
    x_close_display: unsafe extern "C" fn(*mut std::ffi::c_void) -> std::os::raw::c_int,
    x_keysym_to_keycode: unsafe extern "C" fn(*mut std::ffi::c_void, std::os::raw::c_ulong) -> std::os::raw::c_uchar,
    x_query_keymap: unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::os::raw::c_char) -> std::os::raw::c_int,
}

unsafe impl Send for X11Lib {}
unsafe impl Sync for X11Lib {}

fn load_x11() -> Option<X11Lib> {
    unsafe {
        // Try standard filename first
        let lib_name = std::ffi::CString::new("libX11.so.6").ok()?;
        let mut handle = dlopen(lib_name.as_ptr(), RTLD_LAZY);
        if handle.is_null() {
            // Fallback to unversioned filename
            let lib_name_alt = std::ffi::CString::new("libX11.so").ok()?;
            handle = dlopen(lib_name_alt.as_ptr(), RTLD_LAZY);
            if handle.is_null() {
                return None;
            }
        }

        let load_sym = |sym_name: &str| -> Option<*mut std::ffi::c_void> {
            let c_str = std::ffi::CString::new(sym_name).ok()?;
            let sym = dlsym(handle, c_str.as_ptr());
            if sym.is_null() { None } else { Some(sym) }
        };

        let x_open_display = std::mem::transmute(load_sym("XOpenDisplay")?);
        let x_close_display = std::mem::transmute(load_sym("XCloseDisplay")?);
        let x_keysym_to_keycode = std::mem::transmute(load_sym("XKeysymToKeycode")?);
        let x_query_keymap = std::mem::transmute(load_sym("XQueryKeymap")?);

        Some(X11Lib {
            x_open_display,
            x_close_display,
            x_keysym_to_keycode,
            x_query_keymap,
        })
    }
}

/// Polls the local X11 server to get the current physical modifier keys state.
/// Returns `None` if X11 is not available (headless, Wayland-only, SSH without forwarding, etc.).
pub fn get_x11_modifiers() -> Option<KeyModifiers> {
    static X11_LIB: OnceLock<Option<X11Lib>> = OnceLock::new();
    let lib = X11_LIB.get_or_init(load_x11).as_ref()?;

    unsafe {
        let display = (lib.x_open_display)(std::ptr::null());
        if display.is_null() {
            return None;
        }

        let code_ctrl_l = (lib.x_keysym_to_keycode)(display, XK_CONTROL_L);
        let code_ctrl_r = (lib.x_keysym_to_keycode)(display, XK_CONTROL_R);
        let code_alt_l = (lib.x_keysym_to_keycode)(display, XK_ALT_L);
        let code_alt_r = (lib.x_keysym_to_keycode)(display, XK_ALT_R);
        let code_shift_l = (lib.x_keysym_to_keycode)(display, XK_SHIFT_L);
        let code_shift_r = (lib.x_keysym_to_keycode)(display, XK_SHIFT_R);

        let mut keys: [std::os::raw::c_char; 32] = [0; 32];
        (lib.x_query_keymap)(display, keys.as_mut_ptr());

        (lib.x_close_display)(display);

        let keys_u8 = std::mem::transmute::<[std::os::raw::c_char; 32], [u8; 32]>(keys);

        let is_pressed = |keycode: u8| -> bool {
            if keycode == 0 {
                return false;
            }
            let byte_idx = (keycode / 8) as usize;
            let bit_idx = keycode % 8;
            (keys_u8[byte_idx] & (1 << bit_idx)) != 0
        };

        let mut modifiers = KeyModifiers::empty();
        if is_pressed(code_ctrl_l) || is_pressed(code_ctrl_r) {
            modifiers |= KeyModifiers::CONTROL;
        }
        if is_pressed(code_alt_l) || is_pressed(code_alt_r) {
            modifiers |= KeyModifiers::ALT;
        }
        if is_pressed(code_shift_l) || is_pressed(code_shift_r) {
            modifiers |= KeyModifiers::SHIFT;
        }

        Some(modifiers)
    }
}
