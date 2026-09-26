mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc};
use std::thread;

use concir_sync::Semaphore;

/// Owns shared resource `a`.
mod module_a {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc};use crate::cir_trace::sync::{Mutex};

    pub struct ModuleA {
        pub a: Arc<Mutex<i32>>,
    }

    impl ModuleA {
        pub fn new() -> Self {
            Self {
                a: Arc::new(Mutex::new(0)),
            }
        }
    }

    /// Task t1, running in the module that owns `a`.
    /// Declares a dependency on `b`, the resource owned by the other module.
    /// Both tasks acquire `a` before `b`, so neither can wait forever while
    /// holding the resource the other needs.
    pub fn t1(a: Arc<Mutex<i32>>, b: Arc<Mutex<i32>>, finished: Arc<AtomicUsize>) {
        let mut ga = a.lock().unwrap();
        let mut gb = b.lock().unwrap();
        *ga += 1;
        *gb += 1;
        finished.fetch_add(1, Ordering::SeqCst);
        drop(gb);
        drop(ga);
    }
}

/// Owns shared resource `b`.
mod module_b {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc};use crate::cir_trace::sync::{Mutex};

    pub struct ModuleB {
        pub b: Arc<Mutex<i32>>,
    }

    impl ModuleB {
        pub fn new() -> Self {
            Self {
                b: Arc::new(Mutex::new(0)),
            }
        }
    }

    /// Task t2, running in the module that owns `b`.
    /// Declares a dependency on `a`, the resource owned by the other module.
    /// Lock order matches t1: `a` then `b`.
    pub fn t2(a: Arc<Mutex<i32>>, b: Arc<Mutex<i32>>, finished: Arc<AtomicUsize>) {
        let mut ga = a.lock().unwrap();
        let mut gb = b.lock().unwrap();
        *ga += 1;
        *gb += 1;
        finished.fetch_add(1, Ordering::SeqCst);
        drop(gb);
        drop(ga);
    }
}

fn main() { cir_trace::init();
    let _admission = Semaphore::new_named("_admission_semaphore0", 2);

    let owned_a = module_a::ModuleA::new();
    let owned_b = module_b::ModuleB::new();
    let finished = Arc::new(AtomicUsize::new(0));

    let a_for_t1 = Arc::clone(&owned_a.a);
    let b_for_t1 = Arc::clone(&owned_b.b);
    let finished_t1 = Arc::clone(&finished);

    let a_for_t2 = Arc::clone(&owned_a.a);
    let b_for_t2 = Arc::clone(&owned_b.b);
    let finished_t2 = Arc::clone(&finished);

    let t1 = cir_trace::spawn("t1", move || {
        module_a::t1(a_for_t1, b_for_t1, finished_t1);
    });
    let t2 = cir_trace::spawn("t2", move || {
        module_b::t2(a_for_t2, b_for_t2, finished_t2);
    });

    t1.join().unwrap();
    t2.join().unwrap();

    let done = usize::from(finished.load(Ordering::SeqCst) == 2);
    println!("DONE done={done}");
 cir_trace::finish();}
