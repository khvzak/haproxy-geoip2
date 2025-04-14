use std::ops::Deref;
use std::path::PathBuf;
use std::sync::atomic::Ordering::{Acquire, Relaxed};
use std::sync::atomic::{AtomicU64, AtomicU8};
use std::sync::Arc;

use arc_swap::ArcSwapOption;
use haproxy_api::{Core, LogLevel};
use maxminddb::Reader;
use mlua::Lua;

// Database status
enum DbStatus {
    New = 0,
    Loading,
    Loaded,
    ErrorNew,
    Error,
}

const DB_NEW: u8 = DbStatus::New as u8;
const DB_LOADING: u8 = DbStatus::Loading as u8;
const DB_LOADED: u8 = DbStatus::Loaded as u8;
const DB_ERROR_NEW: u8 = DbStatus::ErrorNew as u8;
const DB_ERROR: u8 = DbStatus::Error as u8;

pub(crate) struct Database {
    db: ArcSwapOption<Reader<Vec<u8>>>,
    path: ArcSwapOption<PathBuf>,
    status: AtomicU8,
    last_err: ArcSwapOption<String>,
    reload_interval: AtomicU64,
}

impl Deref for Database {
    type Target = ArcSwapOption<Reader<Vec<u8>>>;

    fn deref(&self) -> &Self::Target {
        &self.db
    }
}

impl Database {
    pub(crate) const fn new() -> Self {
        Self {
            db: ArcSwapOption::const_empty(),
            path: ArcSwapOption::const_empty(),
            status: AtomicU8::new(DB_NEW),
            last_err: ArcSwapOption::const_empty(),
            reload_interval: AtomicU64::new(0),
        }
    }

    pub(crate) fn configure(&'static self, path: PathBuf, reload_interval: u64) {
        self.path.store(Some(Arc::new(path)));
        self.reload_interval.store(reload_interval, Relaxed);
    }

    /// Checks database status, loads it if needed
    pub(crate) fn check_status(&'static self, lua: &Lua) {
        match (self.status).compare_exchange(DB_NEW, DB_LOADING, Acquire, Relaxed) {
            Ok(_) => self.spawn_reload(),
            Err(DB_ERROR_NEW) => {
                if (self.status)
                    .compare_exchange(DB_ERROR_NEW, DB_ERROR, Acquire, Relaxed)
                    .is_ok()
                {
                    let err = self.last_err.load();
                    if let Some(err) = err.as_ref() {
                        let path = self.path();
                        let msg = format!("Error loading database '{}': {err}", path.display());
                        if let Ok(core) = Core::new(lua) {
                            _ = core.log(LogLevel::Err, msg);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// (Re)Loads database
    pub(crate) fn reload(&'static self) {
        let path = self.path();
        match Reader::open_readfile(&*path) {
            Ok(reader) => {
                self.db.store(Some(Arc::new(reader)));
                self.status.store(DB_LOADED, Relaxed);
            }
            Err(err) => {
                self.last_err.store(Some(Arc::new(err.to_string())));
                self.status.store(DB_ERROR_NEW, Relaxed);
            }
        }
    }

    /// (Re)Load database, optionally in background and returning immediately
    pub(crate) fn spawn_reload(&'static self) {
        if self.reload_interval() == 0 {
            self.reload();
        } else {
            std::thread::spawn(move || self.reload());
        }
    }

    pub(crate) fn trigger_reload(&'static self) {
        let trigger = (self.status)
            .compare_exchange(DB_LOADED, DB_LOADING, Acquire, Relaxed)
            .or_else(|_| (self.status).compare_exchange(DB_ERROR, DB_LOADING, Acquire, Relaxed))
            .is_ok();
        if trigger {
            self.spawn_reload();
        }
    }

    pub(crate) fn path(&self) -> Arc<PathBuf> {
        (self.path.load().as_ref())
            .expect("Database is not configured")
            .clone()
    }

    pub(crate) fn reload_interval(&self) -> u64 {
        self.reload_interval.load(Relaxed)
    }
}
