use std::sync::{Arc, Mutex};
use std::thread;

struct ModuleA {
    a: Arc<Mutex<()>>,
}

struct ModuleB {
    b: Arc<Mutex<()>>,
}

impl ModuleA {
    fn new(a: Arc<Mutex<()>>) -> Self {
        ModuleA { a }
    }

    fn t1(&self, b: Arc<Mutex<()>>) {
        // Acquire in a consistent global order: a then b.
        let _ga = self.a.lock().unwrap();
        let _gb = b.lock().unwrap();
        // Hold both resources while performing work.
        // (No-op work; resources held simultaneously.)
        // Both guards drop here, releasing resources before finishing.
    }
}

impl ModuleB {
    fn new(b: Arc<Mutex<()>>) -> Self {
        ModuleB { b }
    }

    fn t2(&self, a: Arc<Mutex<()>>) {
        // Acquire in the same global order: a then b.
        let _ga = a.lock().unwrap();
        let _gb = self.b.lock().unwrap();
        // Hold both resources while performing work.
        // Both guards drop here, releasing resources before finishing.
    }
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let module_a = ModuleA::new(Arc::clone(&a));
    let module_b = ModuleB::new(Arc::clone(&b));

    let a_for_t2 = Arc::clone(&a);
    let b_for_t1 = Arc::clone(&b);

    let h1 = thread::spawn(move || {
        module_a.t1(b_for_t1);
    });

    let h2 = thread::spawn(move || {
        module_b.t2(a_for_t2);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}
