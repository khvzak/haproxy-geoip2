use std::net::IpAddr;

use maxminddb::geoip2::city::Country;
use maxminddb::geoip2::City;
use mlua::prelude::{IntoLua, Lua, LuaValue};

use crate::db::Database;
use crate::GeoValue;

// Global maxmind city database shared between all workers
pub(crate) static DB: Database = Database::new();

pub(crate) fn lookup<'a>(lua: &'a Lua, ip: IpAddr, props: &[String]) -> Option<LuaValue<'a>> {
    DB.check_status(lua);

    let db = DB.load();
    let reader = db.as_ref()?;
    let city = reader.lookup::<City>(ip).ok()?;
    lookup_city(&city, props).and_then(|v| v.into_lua(lua).ok())
}

fn lookup_city<'a>(city: &'a City, props: &[String]) -> Option<GeoValue<'a>> {
    match props.get(0)?.as_str() {
        "city" => match props.get(1).map(|s| s.as_str()) {
            Some("names") => {
                if let Some(city) = city.city.as_ref().and_then(|c| c.names.as_ref()) {
                    if let Some(&value) = props.get(2).and_then(|lang| city.get(lang.as_str())) {
                        return Some(GeoValue::Str(value));
                    }
                }
            }
            Some(_) => {}
            None => {
                if let Some(&value) = (city.city.as_ref())
                    .and_then(|c| c.names.as_ref())
                    .and_then(|names| names.get("en"))
                {
                    return Some(GeoValue::Str(value));
                }
            }
        },
        "country" => {
            return city
                .country
                .as_ref()
                .and_then(|c| lookup_country(c, &props[1..]));
        }
        "registered_country" => {
            return city
                .registered_country
                .as_ref()
                .and_then(|c| lookup_country(c, &props[1..]));
        }
        "location" => match props.get(1).map(|s| s.as_str()) {
            Some("latitude") => {
                if let Some(lat) = city.location.as_ref().and_then(|l| l.latitude) {
                    return Some(GeoValue::Float(lat));
                }
            }
            Some("longitude") => {
                if let Some(lon) = city.location.as_ref().and_then(|l| l.longitude) {
                    return Some(GeoValue::Float(lon));
                }
            }
            Some("timezone") => {
                if let Some(tz) = city.location.as_ref().and_then(|l| l.time_zone) {
                    return Some(GeoValue::Str(tz));
                }
            }
            Some("metro_code") => {
                if let Some(code) = city.location.as_ref().and_then(|l| l.metro_code) {
                    return Some(GeoValue::UInt(code as u32));
                }
            }
            _ => {}
        },
        "postal" => match props.get(1).map(|s| s.as_str()) {
            Some("code") | None => {
                if let Some(code) = city.postal.as_ref().and_then(|p| p.code) {
                    return Some(GeoValue::Str(code));
                }
            }
            _ => {}
        },
        "continent" => match props.get(1).map(|s| s.as_str()) {
            Some("code") | None => {
                if let Some(code) = city.continent.as_ref().and_then(|c| c.code) {
                    return Some(GeoValue::Str(code));
                }
            }
            Some("names") => {
                if let Some(continent) = city.continent.as_ref().and_then(|c| c.names.as_ref()) {
                    if let Some(&value) = props.get(2).and_then(|lang| continent.get(lang.as_str()))
                    {
                        return Some(GeoValue::Str(value));
                    }
                }
            }
            _ => {}
        },
        "subdivision" => {
            if let Some(subdivisions) = city.subdivisions.as_ref() {
                if let Some(index) = props.get(1).and_then(|s| s.parse::<usize>().ok()) {
                    if let Some(subdivision) = subdivisions.get(index) {
                        match props.get(2).map(|s| s.as_str()) {
                            Some("names") => {
                                if let Some(names) = subdivision.names.as_ref() {
                                    if let Some(&value) =
                                        props.get(3).and_then(|lang| names.get(lang.as_str()))
                                    {
                                        return Some(GeoValue::Str(value));
                                    }
                                }
                            }
                            Some("iso_code") | None => {
                                if let Some(code) = subdivision.iso_code {
                                    return Some(GeoValue::Str(code));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        "traits" => {
            if let Some(traits) = city.traits.as_ref() {
                match props.get(1).map(|s| s.as_str()) {
                    Some("is_anonymous_proxy") => {
                        if let Some(is_anonymous_proxy) = traits.is_anonymous_proxy {
                            return Some(GeoValue::Bool(is_anonymous_proxy));
                        }
                    }
                    Some("is_anycast") => {
                        if let Some(is_anycast) = traits.is_anycast {
                            return Some(GeoValue::Bool(is_anycast));
                        }
                    }
                    Some("is_satellite_provider") => {
                        if let Some(is_satellite_provider) = traits.is_satellite_provider {
                            return Some(GeoValue::Bool(is_satellite_provider));
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    None
}

fn lookup_country<'a>(country: &'a Country, props: &[String]) -> Option<GeoValue<'a>> {
    match props.get(0).map(|s| s.as_str()) {
        Some("names") => {
            if let Some(country) = country.names.as_ref() {
                if let Some(&value) = props.get(2).and_then(|lang| country.get(lang.as_str())) {
                    return Some(GeoValue::Str(value));
                }
            }
        }
        Some("iso_code") | None => {
            if let Some(code) = country.iso_code {
                return Some(GeoValue::Str(code));
            }
        }
        Some("is_in_european_union") => {
            if let Some(is_eu) = country.is_in_european_union {
                return Some(GeoValue::Bool(is_eu));
            }
        }
        _ => {}
    }
    None
}
