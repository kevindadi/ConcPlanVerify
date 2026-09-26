use std::sync::{Arc, Mutex};
use std::thread;

use concir_sync::Semaphore;

/// Owns shared resource `a`.
mod module_a {
    use std::sync::{Arc, Mutex};

    pub fn create_a() -> Arc<Mutex<()>> {
        Arc::new(Mutex::new(()))
    }

    /// Task t1 depends on `b`, the resource owned by the other module.
    /// Global lock order is `a` then `b`.
    pub fn t1(a: &Mutex<()>, b: &Mutex<()>, gate: &concir_sync::Semaphore) {
        let permit = gate.acquire();
        let ga = a.lock().unwrap();
        let gb = b.lock().unwrap();
        // Work while both resources are held.
        let _held = (&ga, &gb);
        drop(gb);
        drop(ga);
        drop(permit);
    }
}

/// Owns shared resource `b`.
mod module_b {
    use std::sync::{Arc, Mutex};

    pub fn create_b() -> Arc<Mutex<()>> {
        Arc::new(Mutex::new(()))
    }

    /// Task t2 depends on `a`, the resource owned by the other module.
    /// Global lock order is `a` then `b`.
    pub fn t2(a: &Mutex<()>, b: &Mutex<()>, gate: &concir_sync::Semaphore) {
        let permit = gate.acquire();
        let ga = a.lock().unwrap();
        let gb = b.lock().unwrap();
        // Work while both resources are held.
        let _held = (&ga, &gb);
        drop(gb);
        drop(ga);
        drop(permit);
    }
}

fn main() {
    let a = module_a::create_a();
    let b = module_b::create_b();
    // Two permits: both tasks may enter. The shared lock order prevents a cycle.
    let gate = Semaphore::new(2);

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let g1 = Arc::clone(&gate);
    let h1 = thread::spawn(move || {
        module_a::t1(&a1, &b1, &g1);
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let g2 = Arc::clone(&gate);
    let h2 = thread::spawn(move || {
        module_b::t2(&a2, &b2, &g2);
    });

    h1.join().unwrap();
    h2.join().unwrap();
    println!("DONE done=1");
}
