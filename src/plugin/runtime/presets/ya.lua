-- ya.lua — Pairee plugin-runtime presets loaded into every plugin
-- sandbox at load time. The names follow the reference plugin system
-- the M2 work is modelled on, but no third-party code is vendored
-- here. Add small runtime conveniences that the userdata surface
-- cannot express directly in Rust.
--
-- Today this file is just a marker; concrete helpers will land here
-- in M3/M4 (e.g. a `for _, entry in entries(...)` iterator that
-- uses the new Entries helper).
--
-- Keep this file pure Lua; it runs in the plugin's Lua VM.

-- Marker module.
return {}
