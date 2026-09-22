use std::sync::{Arc, Mutex};
use std::thread;

// Module 1 owns resource `a`.
mod module1 {
    use std::sync::{Arc, Mutex};

    pub struct ResourceA {
        pub lock: Mutex<()>,
    }

    impl ResourceA {
        pub fn new() -> Self {
            ResourceA {
                lock: Mutex::new(()),
            }
        }
    }

    // t1 runs in module1, depends on resource b owned by module2.
    pub fn t1(a: Arc<ResourceA>, b: Arc<crate::module2::ResourceB>) {
        // Acquire both resources in a consistent global order to avoid deadlock.
        let _guard_a = a.lock.lock().unwrap();
        let _guard_b = b.lock.lock().unwrap();

        // Hold both resources while performing work.
        // (work)

        // Guards are dropped here, releasing both resources before finishing.
    }
}

// Module 2 owns resource `b`.
mod module2 {
    use std::sync::{Arc, Mutex};

    pub struct ResourceB {
        pub lock: Mutex<()>,
    }

    impl ResourceB {
        pub fn new() -> Self {
            ResourceB {
                lock: Mutex::new(()),
            }
        }
    }

    // t2 runs in module2, depends on resource a owned by module1.
    pub fn t2(b: Arc<ResourceB>, a: Arc<crate::module1::ResourceA>) {
        // Acquire both resources in the same consistent global order to avoid deadlock.
        let _guard_a = a.lock.lock().unwrap();
        let _guard_b = b.lock.lock().unwrap();

        // Hold both resources while performing work.
        // (work)

        // Guards are dropped here, releasing both resources before finishing.
    }
}

fn main() {
    let a = Arc::new(module1::ResourceA::new());
    let b = Arc::new(module2::ResourceB::new());

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);

    // Starting thread launches both tasks and only finishes after both have finished.
    let handle1 = thread::spawn(move || {
        module1::t1(a1, b1);
    });
    let handle2 = thread::spawn(move || {
        module2::t2(b2, a2);
    });

    handle1.join().unwrap();
    handle2.join().unwrap();

    println!("DONE done=1");
}
