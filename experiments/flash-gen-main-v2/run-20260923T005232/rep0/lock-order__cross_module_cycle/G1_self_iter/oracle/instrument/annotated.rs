mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Module 1 owns resource `a`.
mod module1 {
    use std::sync::{Arc};

    pub struct ResourceA {
        pub lock: Mutex<()>,
    }

    impl ResourceA {
        pub fn new() -> Arc<Self> {
            Arc::new(ResourceA {
                lock: Mutex::new(()),
            })
        }
    }
}

// Module 2 owns resource `b`.
mod module2 {
    use std::sync::{Arc};

    pub struct ResourceB {
        pub lock: Mutex<()>,
    }

    impl ResourceB {
        pub fn new() -> Arc<Self> {
            Arc::new(ResourceB {
                lock: Mutex::new(()),
            })
        }
    }
}

use module1::ResourceA;
use module2::ResourceB;

// Task t1 runs in module1, depends on resource b owned by module2.
fn t1(a: Arc<ResourceA>, b: Arc<ResourceB>) {
    // Acquire in a consistent global order to avoid deadlock.
    let _ga = a.lock.lock().unwrap();
    let _gb = b.lock.lock().unwrap();
    // Hold both resources while performing work.
    // (work)
    // Resources released automatically at end of scope.
}

// Task t2 runs in module2, depends on resource a owned by module1.
fn t2(a: Arc<ResourceA>, b: Arc<ResourceB>) {
    // Same global order: a then b.
    let _ga = a.lock.lock().unwrap();
    let _gb = b.lock.lock().unwrap();
    // Hold both resources while performing work.
    // (work)
    // Resources released automatically at end of scope.
}

fn main() { cir_trace::init();
    let a = ResourceA::new();
    let b = ResourceB::new();

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);

    let h1 = cir_trace::spawn("h1", move || t1(a1, b1));
    let h2 = cir_trace::spawn("h2", move || t2(a2, b2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
