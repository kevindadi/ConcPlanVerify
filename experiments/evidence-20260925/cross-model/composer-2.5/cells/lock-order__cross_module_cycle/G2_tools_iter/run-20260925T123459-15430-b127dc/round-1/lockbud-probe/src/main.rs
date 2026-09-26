use concir_sync::Semaphore;
use std::sync::{Arc, Mutex};
use std::thread;

mod module_a {
    use super::*;

    pub fn create_a() -> Arc<Mutex<()>> {
        Arc::new(Mutex::new(()))
    }

    /// t1 — depends on resource b (owned by module_b).
    pub fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>, gate: Arc<Semaphore>) {
        let _gate = gate.acquire();
        let _a = a.lock().unwrap();
        let _b = b.lock().unwrap();
        // work while holding both a and b
        drop(_b);
        drop(_a);
        drop(_gate);
    }
}

mod module_b {
    use super::*;

    pub fn create_b() -> Arc<Mutex<()>> {
        Arc::new(Mutex::new(()))
    }

    /// t2 — depends on resource a (owned by module_a).
    pub fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>, gate: Arc<Semaphore>) {
        let _gate = gate.acquire();
        let _a = a.lock().unwrap();
        let _b = b.lock().unwrap();
        // work while holding both a and b
        drop(_b);
        drop(_a);
        drop(_gate);
    }
}

fn main() {
    let a = module_a::create_a();
    let b = module_b::create_b();
    let gate = Semaphore::new(1);

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let g1 = Arc::clone(&gate);
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let g2 = Arc::clone(&gate);

    let h1 = thread::spawn(move || module_a::t1(a1, b1, g1));
    let h2 = thread::spawn(move || module_b::t2(a2, b2, g2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}
