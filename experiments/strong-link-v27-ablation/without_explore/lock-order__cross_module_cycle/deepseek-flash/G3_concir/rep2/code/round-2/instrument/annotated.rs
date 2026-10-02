mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
mod modA {
    use crate::cir_trace::sync::{Mutex};

    /// Shared resource owned by module modA.
    pub static a: Mutex<()> = Mutex::new_named("a_mutex0", ());

    /// Task t1: locks a (owned here), then b (owned by modB),
    /// uses both resources at the same time, then releases each
    /// resource it holds before returning.
    pub fn t1() {
        let guard_a = a.lock().unwrap();
        let guard_b = crate::modB::b.lock().unwrap();

        let mut x: i32 = 0;
        x = 1;
        let _ = x;

        drop(guard_b);
        drop(guard_a);
    }

    /// Entry scope: launches both tasks and only finishes after
    /// both of them have finished.
    pub fn main() {
        let h1 = cir_trace::spawn("t1#671", || {
            crate::modA::t1();
        });
        let h2 = cir_trace::spawn("t2#755", || {
            crate::modB::t2();
        });

        h1.join().unwrap();
        h2.join().unwrap();

        println!("DONE done=1");
    }
}

mod modB {
    use crate::cir_trace::sync::{Mutex};

    /// Shared resource owned by module modB.
    pub static b: Mutex<()> = Mutex::new_named("b_mutex0", ());

    /// Task t2: locks a (owned by modA), then b (owned here),
    /// uses both resources at the same time, then releases each
    /// resource it holds before returning.
    pub fn t2() {
        let guard_a = crate::modA::a.lock().unwrap();
        let guard_b = b.lock().unwrap();

        let mut y: i32 = 0;
        y = 1;
        let _ = y;

        drop(guard_b);
        drop(guard_a);
    }
}

fn main() { cir_trace::init();
    modA::main();
 cir_trace::finish();}
