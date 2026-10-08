//! Resource limits for plugin Lua states.
//!
//! * A memory cap (mlua allocator limit) stops plugins from exhausting RAM.
//! * An instruction-count hook acts as a watchdog: when Lua code runs
//!   continuously (no return to Rust for longer than [`IDLE_GAP`]) for more
//!   than the configured burst, the plugin is aborted with a runtime error.
//!   Once tripped, the hook fires on every instruction so a `pcall` loop
//!   cannot swallow the error; it relaxes again after the plugin goes idle.
//!
//! The hook is installed on the main Lua thread, which is where Pairee runs
//! plugin callbacks; untrusted plugins have no `coroutine` library.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Memory cap for untrusted plugins.
pub const UNTRUSTED_MEMORY_LIMIT: usize = 128 * 1024 * 1024;

/// Memory cap for trusted plugins.
pub const TRUSTED_MEMORY_LIMIT: usize = 512 * 1024 * 1024;

/// Longest uninterrupted Lua execution allowed before the plugin is aborted.
pub const MAX_CONTINUOUS_RUN: Duration = Duration::from_secs(10);

/// A pause between watchdog checks longer than this means Lua returned to
/// Rust (or waited on I/O); the next run starts a fresh budget.
pub const IDLE_GAP: Duration = Duration::from_millis(50);

/// Instructions between watchdog checks in normal operation.
const CHECK_EVERY: u32 = 100_000;

#[derive(Clone, Copy)]
struct WatchdogConfig {
    max_run: Duration,
    idle_gap: Duration,
}

struct Watchdog {
    config: WatchdogConfig,
    run_start: Instant,
    last_tick: Instant,
    tripped: bool,
}

enum Verdict {
    Ok,
    /// Went idle after being tripped: restore the cheap hook.
    Recovered,
    /// Limit just exceeded: switch to per-instruction checks.
    Tripped,
    StillTripped,
}

impl Watchdog {
    fn new(config: WatchdogConfig) -> Self {
        let now = Instant::now();
        Self {
            config,
            run_start: now,
            last_tick: now,
            tripped: false,
        }
    }

    fn tick(&mut self, now: Instant) -> Verdict {
        let idle = now.duration_since(self.last_tick) > self.config.idle_gap;
        self.last_tick = now;
        if idle {
            self.run_start = now;
            let was_tripped = std::mem::replace(&mut self.tripped, false);
            return if was_tripped {
                Verdict::Recovered
            } else {
                Verdict::Ok
            };
        }
        if self.tripped {
            return Verdict::StillTripped;
        }
        if now.duration_since(self.run_start) > self.config.max_run {
            self.tripped = true;
            return Verdict::Tripped;
        }
        Verdict::Ok
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
        log::warn!("Could not set plugin memory limit: {e}");
    }
    install_watchdog(
        lua,
        WatchdogConfig {
            max_run: MAX_CONTINUOUS_RUN,
            idle_gap: IDLE_GAP,
        },
    );
}

fn install_watchdog(lua: &mlua::Lua, config: WatchdogConfig) {
    set_hook(
        lua,
        Arc::new(Mutex::new(Watchdog::new(config))),
        CHECK_EVERY,
    );
}

fn set_hook(lua: &mlua::Lua, state: Arc<Mutex<Watchdog>>, every: u32) {
    let triggers = mlua::HookTriggers::new().every_nth_instruction(every);
    lua.set_hook(triggers, move |lua, _debug| {
        let verdict = match state.lock() {
            Ok(mut w) => w.tick(Instant::now()),
            Err(_) => return Ok(()),
        };
        match verdict {
            Verdict::Ok => Ok(()),
            Verdict::Recovered => {
                set_hook(lua, Arc::clone(&state), CHECK_EVERY);
                Ok(())
            }
            Verdict::Tripped => {
                set_hook(lua, Arc::clone(&state), 1);
                Err(limit_error())
            }
            Verdict::StillTripped => Err(limit_error()),
        }
    });
}

fn limit_error() -> mlua::Error {
    mlua::Error::RuntimeError("plugin aborted: exceeded its execution time limit".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_lua() -> mlua::Lua {
        let lua = mlua::Lua::new();
        install_watchdog(
            &lua,
            WatchdogConfig {
                max_run: Duration::from_millis(200),
                idle_gap: Duration::from_millis(50),
            },
        );
        lua
    }

    #[test]
    fn infinite_loop_is_aborted() {
        let lua = test_lua();
        let err = lua.load("while true do end").exec().unwrap_err();
        assert!(err.to_string().contains("execution time limit"), "{err}");
    }

    #[test]
    fn pcall_cannot_swallow_the_abort() {
        let lua = test_lua();
        let err = lua
            .load("while true do pcall(function() while true do end end) end")
            .exec()
            .unwrap_err();
        assert!(err.to_string().contains("execution time limit"), "{err}");
    }

    #[test]
    fn plugin_recovers_after_going_idle() {
        let lua = test_lua();
        assert!(lua.load("while true do end").exec().is_err());
        std::thread::sleep(Duration::from_millis(120));
        let v: i64 = lua
            .load("local s = 0 for i = 1, 1000 do s = s + i end return s")
            .eval()
            .unwrap();
        assert_eq!(v, 500500);
    }

    #[test]
    fn memory_limit_is_enforced() {
        let lua = mlua::Lua::new();
        apply(&lua, false);
        let result = lua
            .load("local t = {} for i = 1, 1e9 do t[i] = string.rep('x', 4096) .. i end")
            .exec();
        assert!(
            matches!(result, Err(mlua::Error::MemoryError(_))),
            "{result:?}"
        );
    }
}
