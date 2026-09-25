mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
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
        // Resources released when guards drop at end of scope.
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
        // Resources released when guards drop at end of scope.
    }
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));

    let module_a = Arc::new(ModuleA::new(Arc::clone(&a)));
    let module_b = Arc::new(ModuleB::new(Arc::clone(&b)));

    let a_for_t1 = Arc::clone(&a);
    let b_for_t1 = Arc::clone(&b);
    let a_for_t2 = Arc::clone(&a);
    let b_for_t2 = Arc::clone(&b);

    let ma = Arc::clone(&module_a);
    let mb = Arc::clone(&module_b);

    let h1 = cir_trace::spawn("h1", move || {
        let _ = a_for_t1;
        ma.t1(b_for_t1);
    });

    let h2 = cir_trace::spawn("h2", move || {
        let _ = b_for_t2;
        mb.t2(a_for_t2);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
