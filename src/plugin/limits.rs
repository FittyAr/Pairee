//! Resource limits for plugin Lua states.
//!
//! * A memory cap (mlua allocator limit) stops plugins from exhausting RAM.
//! * A CPU watchdog bounds every call into the plugin by **wall-clock**
//!   time. The Rust dispatcher wraps each call (`main.lua`, `setup`,
//!   `entry`, `peek`, hook callbacks) in [`guarded`], which marks the state
//!   as "inside a Lua call" on entry and clears it on exit. An
//!   instruction-count hook compares the time since entry with
//!   [`MAX_CONTINUOUS_RUN`]; time spent inside C functions such as
//!   `string.rep` counts too, because the clock only restarts when Rust
//!   starts a new call (there is no idle-gap heuristic to reset it).
//!   Once tripped, the hook fires on every instruction so a `pcall` loop
//!   cannot swallow the error, until the guarded call returns.
//!
//! A single C function that never returns to the VM (one gigantic
//! `string.rep` is bounded by the memory cap instead) cannot be
//! interrupted; the hook fires as soon as control is back in Lua.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Memory cap for untrusted plugins.
pub const UNTRUSTED_MEMORY_LIMIT: usize = 128 * 1024 * 1024;

/// Memory cap for trusted plugins.
pub const TRUSTED_MEMORY_LIMIT: usize = 512 * 1024 * 1024;

/// Longest a single call into a plugin may run before it is aborted.
pub const MAX_CONTINUOUS_RUN: Duration = Duration::from_secs(10);

/// Instructions between watchdog checks in normal operation.
const CHECK_EVERY: u32 = 1_000;

#[derive(Default)]
struct CallState {
    /// Nesting depth of guarded calls (a binding may call back into Lua).
    depth: u32,
    /// When the outermost guarded call started.
    started: Option<Instant>,
    tripped: bool,
}

/// Watchdog shared by the hook and [`guarded`] (stored as Lua app data).
#[derive(Clone)]
struct Watchdog {
    max_run: Duration,
    state: Arc<Mutex<CallState>>,
}

enum Verdict {
    Ok,
    /// Limit just exceeded: switch to per-instruction checks.
    Tripped,
    StillTripped,
}

impl Watchdog {
    fn new(max_run: Duration) -> Self {
        Self {
            max_run,
            state: Arc::default(),
        }
    }

    fn enter(&self) {
        if let Ok(mut s) = self.state.lock() {
            if s.depth == 0 {
                s.started = Some(Instant::now());
                s.tripped = false;
            }
            s.depth += 1;
        }
    }

    /// Leaves a guarded call; `true` when the outermost one ended tripped.
    fn exit(&self) -> bool {
        let Ok(mut s) = self.state.lock() else {
            return false;
        };
        s.depth = s.depth.saturating_sub(1);
        if s.depth > 0 {
            return false;
        }
        s.started = None;
        std::mem::replace(&mut s.tripped, false)
    }

    fn tick(&self, now: Instant) -> Verdict {
        let Ok(mut s) = self.state.lock() else {
            return Verdict::Ok;
        };
        if s.tripped {
            return Verdict::StillTripped;
        }
        match s.started {
            Some(start) if now.duration_since(start) > self.max_run => {
                s.tripped = true;
                Verdict::Tripped
            }
            _ => Verdict::Ok,
        }
    }
}

/// Applies the memory cap and the runaway-execution watchdog to `lua`.
pub fn apply(lua: &mlua::Lua, trusted: bool) {
    let limit = if trusted {
        TRUSTED_MEMORY_LIMIT
    } else {
        UNTRUSTED_MEMORY_LIMIT
    };
    if let Err(e) = lua.set_memory_limit(limit) {
        tracing::warn!("Could not set plugin memory limit: {e}");
    }
    install_watchdog(lua, MAX_CONTINUOUS_RUN);
}

fn install_watchdog(lua: &mlua::Lua, max_run: Duration) {
    let watchdog = Watchdog::new(max_run);
    lua.set_app_data(watchdog.clone());
    set_hook(lua, watchdog, CHECK_EVERY);
}

/// Installs the watchdog as the *global* hook: the main thread, the threads
/// mlua creates for async calls and every coroutine Lua code creates (they
/// inherit the hook) all run under it.
fn set_hook(lua: &mlua::Lua, watchdog: Watchdog, every: u32) {
    let triggers = mlua::HookTriggers::new().every_nth_instruction(every);
    let result = lua.set_global_hook(triggers, move |lua, _debug| {
        match watchdog.tick(Instant::now()) {
            Verdict::Ok => Ok(mlua::VmState::Continue),
            Verdict::Tripped => {
                set_hook(lua, watchdog.clone(), 1);
                Err(limit_error())
            }
            Verdict::StillTripped => Err(limit_error()),
        }
    });
    if let Err(e) = result {
        tracing::warn!("Could not install the plugin watchdog: {e}");
    }
}

/// Runs `f` (a call into the plugin's Lua code) under the CPU watchdog:
/// the budget starts now and stops when `f` returns. Every place where
/// Rust runs plugin code goes through here.
pub fn guarded<R>(lua: &mlua::Lua, f: impl FnOnce() -> mlua::Result<R>) -> mlua::Result<R> {
    let watchdog = lua.app_data_ref::<Watchdog>().map(|w| w.clone());
    if let Some(w) = &watchdog {
        w.enter();
    }
    let result = f();
    if let Some(w) = watchdog
        && w.exit()
    {
        // Back to the cheap hook for the next call.
        set_hook(lua, w, CHECK_EVERY);
    }
    result
}

fn limit_error() -> mlua::Error {
    mlua::Error::RuntimeError("plugin aborted: exceeded its execution time limit".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_lua() -> mlua::Lua {
        let lua = mlua::Lua::new();
        install_watchdog(&lua, Duration::from_millis(200));
        lua
    }

    fn run(lua: &mlua::Lua, code: &str) -> mlua::Result<()> {
        guarded(lua, || lua.load(code).exec())
    }

    #[test]
    fn infinite_loop_is_aborted() {
        let lua = test_lua();
        let err = run(&lua, "while true do end").unwrap_err();
        assert!(err.to_string().contains("execution time limit"), "{err}");
    }

    #[test]
    fn work_inside_c_functions_cannot_evade_the_watchdog() {
        let lua = test_lua();
        let started = Instant::now();
        let err = run(&lua, "while true do local s = string.rep('x', 1e6) end").unwrap_err();
        assert!(err.to_string().contains("execution time limit"), "{err}");
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn pcall_cannot_swallow_the_abort() {
        let lua = test_lua();
        let err = run(
            &lua,
            "while true do pcall(function() while true do end end) end",
        )
        .unwrap_err();
        assert!(err.to_string().contains("execution time limit"), "{err}");
    }

    #[test]
    fn coroutines_cannot_evade_the_watchdog() {
        let lua = test_lua();
        let err = run(
            &lua,
            "local co = coroutine.wrap(function() while true do end end) co()",
        )
        .unwrap_err();
        assert!(err.to_string().contains("execution time limit"), "{err}");
    }

    #[test]
    fn async_calls_cannot_evade_the_watchdog() {
        let lua = test_lua();
        let f: mlua::Function = lua
            .load("return function() while true do end end")
            .eval()
            .unwrap();
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let err = guarded(&lua, || rt.block_on(f.call_async::<()>(()))).unwrap_err();
        assert!(err.to_string().contains("execution time limit"), "{err}");
    }

    #[test]
    fn next_call_gets_a_fresh_budget() {
        let lua = test_lua();
        assert!(run(&lua, "while true do end").is_err());
        let v: i64 = guarded(&lua, || {
            lua.load("local s = 0 for i = 1, 1000 do s = s + i end return s")
                .eval()
        })
        .unwrap();
        assert_eq!(v, 500500);
    }

    #[test]
    fn short_calls_never_accumulate() {
        let lua = test_lua();
        for _ in 0..5 {
            run(
                &lua,
                "local t = os and 0 or 0 for i = 1, 1e5 do t = t + 1 end",
            )
            .unwrap();
            std::thread::sleep(Duration::from_millis(60));
        }
    }

    #[test]
    fn memory_limit_is_enforced() {
        let lua = mlua::Lua::new();
        apply(&lua, false);
        let result = guarded(&lua, || {
            lua.load("local t = {} for i = 1, 1e9 do t[i] = string.rep('x', 4096) .. i end")
                .exec()
        });
        assert!(
            matches!(result, Err(mlua::Error::MemoryError(_))),
            "{result:?}"
        );
    }
}
