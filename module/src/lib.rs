use mlua::prelude::*;

#[mlua::lua_module(skip_memory_check)]
fn haproxy_geoip2_module(lua: &Lua) -> LuaResult<LuaTable> {
    let table = lua.create_table()?;
    table.set("register", lua.create_function(haproxy_geoip2::register)?)?;
    Ok(table)
}
