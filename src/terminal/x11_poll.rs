//! Modifier keys held down, read from the local X server (Linux).
//!
//! `libX11` is loaded at run time (no build dependency, nothing to do on a
//! Wayland-only or headless system). The input thread asks on every tick,
//! so the display connection is opened once per thread and reused, with
//! the keycodes of the modifier keys looked up once. A connection whose
//! server went away is dropped (and, after a pause, opened again) instead
//! of being used: Xlib ends the process on an I/O error.

use crossterm::event::KeyModifiers;
use std::cell::RefCell;
use std::os::raw::{c_char, c_int, c_uchar, c_ulong, c_void};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

// X11 Keysym definitions
const XK_SHIFT_L: c_ulong = 0xFFE1;
const XK_SHIFT_R: c_ulong = 0xFFE2;
const XK_CONTROL_L: c_ulong = 0xFFE3;
const XK_CONTROL_R: c_ulong = 0xFFE4;
const XK_ALT_L: c_ulong = 0xFFE9;
const XK_ALT_R: c_ulong = 0xFFEA;

// Unix dynamic loading flags
const RTLD_LAZY: c_int = 1;

/// Wait before opening the display again after it failed or went away.
const REOPEN_AFTER: Duration = Duration::from_secs(5);

unsafe extern "C" {
    fn dlopen(filename: *const c_char, flag: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

struct X11Lib {
    x_open_display: unsafe extern "C" fn(*const c_char) -> *mut c_void,
    x_close_display: unsafe extern "C" fn(*mut c_void) -> c_int,
    x_connection_number: unsafe extern "C" fn(*mut c_void) -> c_int,
    x_keysym_to_keycode: unsafe extern "C" fn(*mut c_void, c_ulong) -> c_uchar,
    x_query_keymap: unsafe extern "C" fn(*mut c_void, *mut c_char) -> c_int,
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

        let load_sym = |sym_name: &str| -> Option<*mut c_void> {
            let c_str = std::ffi::CString::new(sym_name).ok()?;
            let sym = dlsym(handle, c_str.as_ptr());
            if sym.is_null() { None } else { Some(sym) }
        };

        let x_open_display = std::mem::transmute::<
            *mut c_void,
            unsafe extern "C" fn(*const c_char) -> *mut c_void,
        >(load_sym("XOpenDisplay")?);
        let x_close_display = std::mem::transmute::<
            *mut c_void,
            unsafe extern "C" fn(*mut c_void) -> c_int,
        >(load_sym("XCloseDisplay")?);
        let x_connection_number = std::mem::transmute::<
            *mut c_void,
            unsafe extern "C" fn(*mut c_void) -> c_int,
        >(load_sym("XConnectionNumber")?);
        let x_keysym_to_keycode = std::mem::transmute::<
            *mut c_void,
            unsafe extern "C" fn(*mut c_void, c_ulong) -> c_uchar,
        >(load_sym("XKeysymToKeycode")?);
        let x_query_keymap = std::mem::transmute::<
            *mut c_void,
            unsafe extern "C" fn(*mut c_void, *mut c_char) -> c_int,
        >(load_sym("XQueryKeymap")?);

        Some(X11Lib {
            x_open_display,
            x_close_display,
            x_connection_number,
            x_keysym_to_keycode,
            x_query_keymap,
        })
    }
}

/// An open display plus the keycodes of the modifier keys on it.
struct Connection {
    lib: &'static X11Lib,
    display: *mut c_void,
    /// (left, right) keycodes per modifier; 0 when the key does not exist.
    keys: [(KeyModifiers, c_uchar, c_uchar); 3],
}

impl Connection {
    fn open(lib: &'static X11Lib) -> Option<Self> {
        // SAFETY: Xlib calls on a display this connection owns; it is
        // only used by the thread that opened it.
        unsafe {
            let display = (lib.x_open_display)(std::ptr::null());
            if display.is_null() {
                return None;
            }
            let code = |keysym| (lib.x_keysym_to_keycode)(display, keysym);
            Some(Self {
                lib,
                display,
                keys: [
                    (
                        KeyModifiers::CONTROL,
                        code(XK_CONTROL_L),
                        code(XK_CONTROL_R),
                    ),
                    (KeyModifiers::ALT, code(XK_ALT_L), code(XK_ALT_R)),
                    (KeyModifiers::SHIFT, code(XK_SHIFT_L), code(XK_SHIFT_R)),
                ],
            })
        }
    }

    /// `false` once the server closed the connection (it hung up, or the
    /// socket reads end-of-file); checked without any Xlib call.
    fn alive(&self) -> bool {
        // SAFETY: `display` is open; `poll` and `recv` only look at its
        // socket (`recv` peeks, so Xlib still reads whatever is there).
        unsafe {
            let fd = (self.lib.x_connection_number)(self.display);
            let mut pfd = libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            };
            if libc::poll(&mut pfd, 1, 0) < 0 {
                return false;
            }
            if pfd.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0 {
                return false;
            }
            if pfd.revents & libc::POLLIN != 0 {
                let mut byte = 0u8;
                let peeked = libc::recv(
                    fd,
                    (&mut byte as *mut u8).cast(),
                    1,
                    libc::MSG_PEEK | libc::MSG_DONTWAIT,
                );
                return peeked != 0;
            }
            true
        }
    }

    fn modifiers(&self) -> KeyModifiers {
        let mut keymap = [0 as c_char; 32];
        // SAFETY: `display` is open and `keymap` has the 32 bytes
        // XQueryKeymap fills.
        unsafe { (self.lib.x_query_keymap)(self.display, keymap.as_mut_ptr()) };
        let pressed = |keycode: c_uchar| {
            keycode != 0 && (keymap[usize::from(keycode / 8)] as u8) & (1 << (keycode % 8)) != 0
        };
        self.keys
            .iter()
            .filter(|(_, left, right)| pressed(*left) || pressed(*right))
            .fold(KeyModifiers::empty(), |all, (modifier, _, _)| {
                all | *modifier
            })
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        // SAFETY: closes the display this connection owns, once.
        unsafe { (self.lib.x_close_display)(self.display) };
    }
}

/// This thread's display connection, or when opening it last failed.
enum State {
    Open(Connection),
    Failed(Instant),
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

/// Polls the local X11 server to get the current physical modifier keys state.
/// Returns `None` if X11 is not available (headless, Wayland-only, SSH without forwarding, etc.).
pub fn get_x11_modifiers() -> Option<KeyModifiers> {
    static X11_LIB: OnceLock<Option<X11Lib>> = OnceLock::new();
    let lib = X11_LIB.get_or_init(load_x11).as_ref()?;
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if let Some(State::Open(connection)) = state.as_ref()
            && !connection.alive()
        {
            // Any Xlib call (even closing it) on a dead connection ends
            // the process: leak it.
            if let Some(State::Open(dead)) = state.take() {
                std::mem::forget(dead);
            }
            *state = Some(State::Failed(Instant::now()));
        }
        let retry = match state.as_ref() {
            None => true,
            Some(State::Failed(at)) => at.elapsed() >= REOPEN_AFTER,
            Some(State::Open(_)) => false,
        };
        if retry {
            *state = Some(match Connection::open(lib) {
                Some(connection) => State::Open(connection),
                None => State::Failed(Instant::now()),
            });
        }
        match state.as_ref() {
            Some(State::Open(connection)) => Some(connection.modifiers()),
            _ => None,
        }
    })
}
