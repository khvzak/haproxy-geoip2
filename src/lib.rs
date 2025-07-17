use std::net::IpAddr;

use haproxy_api::Core;
use mlua::prelude::{IntoLua, Lua, LuaFunction, LuaResult, LuaTable, LuaValue};
use mlua::Variadic;

#[derive(Debug, Clone)]
pub(crate) enum GeoValue<'a> {
    Str(&'a str),
    Float(f64),
    UInt(u32),
    Bool(bool),
}

impl IntoLua for GeoValue<'_> {
    fn into_lua(self, lua: &Lua) -> LuaResult<LuaValue> {
        match self {
            GeoValue::Str(s) => s.into_lua(lua),
            GeoValue::Float(f) => f.into_lua(lua),
            GeoValue::UInt(u) => u.into_lua(lua),
            GeoValue::Bool(b) => b.into_lua(lua),
        }
    }
}

/// Register GeoIP2 lookups in the haproxy
///
/// This function registers the GeoIP2 lookups converters in the haproxy Lua environment,
/// including task to reload the databases at a given interval.
///
/// The following Lua options are supported:
///
/// - `reload_interval`: Interval in seconds to reload the databases. Default is 0.
/// - `db.city`: Path to the MaxMind GeoIP2 City database.
/// - `db.asn`: Path to the MaxMind GeoIP2 ASN database.
///
/// # Example
/// ```lua
/// geoip2.register({
///    reload_interval = 86400, -- 1 day
///    db = {
///        city = "/path/to/GeoLite2-City.mmdb",
///         asn = "/path/to/GeoLite2-ASN.mmdb",
///     },
/// })
/// ```
pub fn register(lua: &Lua, options: LuaTable) -> LuaResult<()> {
    let core = Core::new(lua)?;

    // Parse options
    let reload_interval: u64 = options.get("reload_interval").unwrap_or(0);

    // Register databases
    if let Ok(db) = options.get::<LuaTable>("db") {
        if let Ok(path) = db.get::<String>("city") {
            city::DB.configure(path.into(), reload_interval);
            register_converter(&core, &city::DB, "city", city::lookup)?;
        }

        if let Ok(asn_path) = db.get::<String>("asn") {
            asn::DB.configure(asn_path.into(), reload_interval);
            register_converter(&core, &asn::DB, "asn", asn::lookup)?;
        }
    }

    Ok(())
}

// `F` is zero-sized, so it's safe to send it across threads
fn register_converter<F>(
    core: &Core,
    db: &'static db::Database,
    prefix: &str,
    lookup: F,
) -> LuaResult<()>
where
    F: Fn(&Lua, IpAddr, &[String]) -> Option<LuaValue> + Send + Copy + 'static,
{
    // Trigger dummy lookup within a worker to load the database
    core.register_task(move |lua| {
        lookup(lua, "0.0.0.0".parse()?, &[]);
        Ok(())
    })?;

    // Register reload task
    let interval = db.reload_interval();
    if interval > 0 {
        let trigger_reload = LuaFunction::wrap(|| {
            db.trigger_reload();
            Ok(())
        });
        core.register_lua_task(mlua::chunk! {
            if core.thread <= 1 then
                while true do
                    core.sleep($interval)
                    $trigger_reload()
                end
            end
        })?;
    }

    core.register_converters(
        &format!("geoip2-lookup-{prefix}"),
        move |lua, (ip, props): (String, Variadic<String>)| {
            let ip = ip.parse::<IpAddr>()?;
            lookup(lua, ip, &props).map_or_else(|| "".into_lua(lua), Ok)
        },
    )
}

mod asn;
mod city;
mod db;
