# haproxy-geoip2

`haproxy-geoip2` adds [MaxMind] GeoIP database support to [HAProxy] 2.8+ Community Edition using [Lua API].

It implemented as a native Lua module written in Rust using [mlua] and [haproxy-api] crates.

[MaxMind]: https://www.maxmind.com
[HAProxy]: https://www.haproxy.org
[Lua API]: https://www.arpalert.org/src/haproxy-lua-api/3.0/index.html
[mlua]: https://github.com/mlua-rs/mlua
[haproxy-api]: https://github.com/khvzak/haproxy-api-rs

## Usage

Please check the [module](module) and [tests](tests) directories for working examples.

```lua
local geoip2 = require("haproxy_geoip2_module")

geoip2.register({
    -- Only City and ASN databases are supported yet
    db = {
        city = "data/GeoIP2-City-Test.mmdb",
        asn = "data/GeoLite2-ASN-Test.mmdb",
    },
    -- How often to reload the database files in seconds
    reload_interval = 86400,
})
```

The module registers the following converters in HAProxy:
```
global
    master-worker
    # This option is required to enable non-blocking databases loading
    insecure-fork-wanted
    lua-load-per-thread haproxy.lua

...

listen http-in
    bind *:8080
    http-request set-var(txn.city) url_param(ip),lua.geoip2-lookup-city("city","names","en")
    http-request set-var(txn.asn) url_param(ip),lua.geoip2-lookup-asn("asn")
    http-request return status 200 content-type text/plain lf-string "{\"city\":\"%[var(txn.city)]\",\"asn\":\"%[var(txn.asn)]\"}"

```

## License

This project is licensed under the [MIT license](LICENSE)
