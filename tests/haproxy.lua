local geoip2 = require("haproxy_geoip2_module")

geoip2.register({
    db = {
        city = "data/GeoIP2-City-Test.mmdb",
        asn = "data/GeoLite2-ASN-Test.mmdb",
    },
    -- How often to reload the database files in seconds
    reload_interval = 30,
})
