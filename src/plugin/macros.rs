//! Macros used by the plugin runtime (M3).
//!
//! Today this module exposes only `runtime_scope!`, which sets the
//! `blocking` flag for the duration of a sync callback and pushes
//! / pops a `RuntimeFrame` on the runtime stack.

use crate::plugin::runtime::runtime::{Runtime, RuntimeFrame};

/// RAII guard that pushes a `RuntimeFrame` on construction and pops
/// it on drop. Used by the `runtime_scope!` macro.
pub struct ScopeGuard<'a> {
    runtime: &'a Runtime,
}

impl<'a> ScopeGuard<'a> {
    pub fn new(runtime: &'a Runtime, frame: RuntimeFrame) -> Self {
        runtime.push_frame(frame);
        Self { runtime }
    }
}

impl<'a> Drop for ScopeGuard<'a> {
    fn drop(&mut self) {
        self.runtime.pop_frame();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::runtime::runtime::Runtime;
    use mlua::Lua;

    #[test]
    fn test_runtime_scope_macro_pushes_and_pops_frame() {
        let lua = Lua::new();
        lua.set_app_data(Runtime::new());
        // Outside the scope, the runtime is non-blocking and empty.
        {
            let rt = lua.app_data_ref::<Runtime>().unwrap();
            assert!(!rt.is_blocking());
            assert_eq!(rt.depth(), 0);
        }
        // Inside the scope, the runtime is blocking and has 1 frame.
        crate::runtime_scope!(lua, "test-frame", {
            let rt = lua.app_data_ref::<Runtime>().unwrap();
            assert!(rt.is_blocking());
            assert_eq!(rt.depth(), 1);
        });
        // After the scope, blocking is cleared and the frame is gone.
        let rt = lua.app_data_ref::<Runtime>().unwrap();
        assert!(!rt.is_blocking());
        assert_eq!(rt.depth(), 0);
    }

    #[test]
    fn test_runtime_scope_creates_runtime_if_missing() {
        let lua = Lua::new();
        // No runtime has been seeded yet.
        assert!(lua.app_data_ref::<Runtime>().is_none());
        crate::runtime_scope!(lua, "auto-init", {
            let rt = lua.app_data_ref::<Runtime>().unwrap();
            assert!(rt.is_blocking());
        });
        // After the scope the runtime persists.
        assert!(lua.app_data_ref::<Runtime>().is_some());
    }
}

/// Run `$body` inside a `Runtime` sync scope. The macro borrows
/// the `Runtime` out of the Lua app data (creating one if missing),
/// pushes a frame with the given `$id`, runs the body, and pops
/// the frame on exit (even if the body panics, via the `Drop`
/// guard).
///
/// Usage:
/// ```ignore
/// let lua: &mlua::Lua = ...;
/// runtime_scope!(lua, "sync-peek", {
///     // … plugin callback body …
/// });
/// ```
#[macro_export]
macro_rules! runtime_scope {
    ($lua:expr, $id:expr, $body:block) => {{
        use $crate::plugin::runtime::runtime::{Runtime, RuntimeFrame};
        // Borrow the shared Runtime, creating one on the app data
        // if this is the first sync call. The `AppDataRef` lives
        // for the duration of the block; the `ScopeGuard` borrows
        // it and pops the frame on drop.
        let app_data: &mlua::Lua = &$lua;
        if app_data.app_data_ref::<Runtime>().is_none() {
            app_data.set_app_data(Runtime::new());
        }
        let runtime = app_data
            .app_data_ref::<Runtime>()
            .expect("Runtime just initialised");
        let _frame_guard = $crate::plugin::macros::ScopeGuard::new(
            &*runtime,
            RuntimeFrame::new($id, true),
        );
        $body
    }};
}
