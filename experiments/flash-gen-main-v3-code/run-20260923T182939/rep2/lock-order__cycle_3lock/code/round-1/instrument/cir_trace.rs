// Generated cir_trace runtime v2 (std only, wrapper types).
#![allow(dead_code)]
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

static EVENTS: OnceLock<std::sync::Mutex<Vec<(String, String, String, String)>>> = OnceLock::new();
static MAIN: OnceLock<std::thread::ThreadId> = OnceLock::new();
static NEXT_TAG: AtomicU64 = AtomicU64::new(1);
static SIDS: OnceLock<std::sync::Mutex<HashMap<String, u64>>> = OnceLock::new();

thread_local! {
    static TAG: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// Pin the main thread's tag to `t0` before any worker can run.
pub fn init() {
    let _ = MAIN.set(std::thread::current().id());
    let _ = EVENTS.get_or_init(|| std::sync::Mutex::new(Vec::new()));
}

/// Thread tag: the eager spawn-order tag when set, `t0` for main, and a lazy
/// fallback for threads created without the helper.
pub fn tag() -> String {
    if let Some(t) = TAG.with(|t| t.borrow().clone()) {
        return t;
    }
    let tid = std::thread::current().id();
    let main = *MAIN.get_or_init(|| tid);
    if tid == main {
        return "t0".to_string();
    }
    let n = NEXT_TAG.fetch_add(1, Ordering::SeqCst);
    let name = format!("t{}", n);
    TAG.with(|t| *t.borrow_mut() = Some(name.clone()));
    name
}

fn next_sid(resource: &str, op: &str) -> String {
    let key = format!("{resource}:{op}");
    let sids = SIDS.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
    let mut map = sids.lock().unwrap();
    let c = map.entry(key.clone()).or_insert(0);
    *c += 1;
    format!("{key}#{c}")
}

pub fn record(op: &str, resource: &str) {
    let sid = next_sid(resource, op);
    let events = EVENTS.get_or_init(|| std::sync::Mutex::new(Vec::new()));
    events
        .lock()
        .unwrap()
        .push((tag(), op.to_string(), resource.to_string(), sid));
}

/// Spawn a worker and record it; the child's completion is witnessed by the
/// program finishing (a finished trace implies `main` returned).
pub fn spawn<F>(name: &'static str, f: F) -> std::thread::JoinHandle<()>
where
    F: FnOnce() + Send + 'static,
{
    record("spawn", name);
    let n = NEXT_TAG.fetch_add(1, Ordering::SeqCst);
    let tag = format!("t{}", n);
    std::thread::spawn(move || {
        TAG.with(|t| *t.borrow_mut() = Some(tag));
        f()
    })
}

pub fn finish() {
    if let Ok(path) = std::env::var("CIR_TRACE_OUT") {
        let events = EVENTS.get_or_init(|| std::sync::Mutex::new(Vec::new()));
        let guard = events.lock().unwrap();
        let mut out = String::new();
        for (t, op, r, s) in guard.iter() {
            out.push_str(&format!(
                "{{\"t\":\"{}\",\"sid\":\"{}\",\"op\":\"{}\",\"r\":\"{}\"}}\n",
                t, s, op, r
            ));
        }
        let _ = std::fs::write(path, out);
    }
}

pub mod sync {
    use super::record;
    use std::ops::{Deref, DerefMut};
    use std::sync::{Condvar as StdCondvar, LockResult, Mutex as StdMutex};

    pub struct Mutex<T> {
        inner: StdMutex<T>,
        name: &'static str,
    }

    pub struct Guard<'a, T> {
        inner: Option<std::sync::MutexGuard<'a, T>>,
        name: &'static str,
        armed: bool,
    }

    impl<T> Mutex<T> {
        pub fn new(value: T) -> Self {
            Self::new_named("mutex", value)
        }
        pub fn new_named(name: &'static str, value: T) -> Self {
            Mutex { inner: StdMutex::new(value), name }
        }
        pub fn lock(&self) -> LockResult<Guard<'_, T>> {
            match self.inner.lock() {
                Ok(g) => {
                    record("mutex_lock", self.name);
                    Ok(Guard { inner: Some(g), name: self.name, armed: true })
                }
                Err(e) => {
                    let g: std::sync::MutexGuard<'_, T> = e.into_inner();
                    record("mutex_lock", self.name);
                    Ok(Guard { inner: Some(g), name: self.name, armed: true })
                }
            }
        }
    }

    impl<'a, T> Deref for Guard<'a, T> {
        type Target = T;
        fn deref(&self) -> &T {
            self.inner.as_ref().unwrap()
        }
    }

    impl<'a, T> DerefMut for Guard<'a, T> {
        fn deref_mut(&mut self) -> &mut T {
            self.inner.as_mut().unwrap()
        }
    }

    impl<'a, T> Drop for Guard<'a, T> {
        fn drop(&mut self) {
            if self.armed {
                record("mutex_unlock", self.name);
            }
        }
    }

    pub struct Condvar {
        inner: StdCondvar,
        name: &'static str,
    }

    impl Default for Condvar {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Condvar {
        pub fn new() -> Self {
            Self::new_named("condvar")
        }
        pub fn new_named(name: &'static str) -> Self {
            Condvar { inner: StdCondvar::new(), name }
        }
        pub fn wait<'a, T>(&self, mut guard: Guard<'a, T>) -> LockResult<Guard<'a, T>> {
            let name = guard.name;
            guard.armed = false;
            let inner: std::sync::MutexGuard<'a, T> =
                guard.inner.take().expect("condvar guard consumed twice");
            // One model step: the release/reacquire around the wait is implicit,
            // so no mutex_unlock/mutex_lock events are emitted here.
            let result = self.inner.wait(inner);
            record("condvar_wait", self.name);
            match result {
                Ok(g) => Ok(Guard { inner: Some(g), name, armed: true }),
                Err(_) => Ok(Guard { inner: None, name, armed: false }),
            }
        }
        pub fn wait_while<'a, T, F>(&self, mut guard: Guard<'a, T>, mut f: F)
            -> LockResult<Guard<'a, T>>
        where
            F: FnMut(&mut T) -> bool,
        {
            let name = guard.name;
            guard.armed = false;
            let inner: std::sync::MutexGuard<'a, T> =
                guard.inner.take().expect("condvar guard consumed twice");
            let result = self.inner.wait_while(inner, |v| f(v));
            record("condvar_wait", self.name);
            match result {
                Ok(g) => Ok(Guard { inner: Some(g), name, armed: true }),
                Err(_) => Ok(Guard { inner: None, name, armed: false }),
            }
        }
        pub fn notify_one(&self) {
            record("condvar_notify", self.name);
            self.inner.notify_one();
        }
        pub fn notify_all(&self) {
            record("condvar_notify_all", self.name);
            self.inner.notify_all();
        }
    }
}

