mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

static WORK: AtomicUsize = AtomicUsize::new(0);

mod first {
    use crate::cir_trace::sync::{Mutex};

    pub static a: Mutex<()> = Mutex::new_named("a_mutex0", ());

    // t1 depends on b, which is owned by the other module.
    pub fn t1(b: &'static Mutex<()>) {
        // Both tasks acquire locks in the same order: a, then b.
        let a_guard = a.lock().unwrap_or_else(|e| e.into_inner());
        let b_guard = b.lock().unwrap_or_else(|e| e.into_inner());

        super::WORK.fetch_add(1, super::Ordering::Relaxed);

        drop(b_guard);
        drop(a_guard);
    }
}

mod second {
    use crate::cir_trace::sync::{Mutex};

    pub static b: Mutex<()> = Mutex::new_named("b_mutex0", ());

    // t2 depends on a, which is owned by the other module.
    pub fn t2(a: &'static Mutex<()>) {
        // Use the same lock order as t1 to prevent deadlock.
        let a_guard = a.lock().unwrap_or_else(|e| e.into_inner());
        let b_guard = b.lock().unwrap_or_else(|e| e.into_inner());

        super::WORK.fetch_add(1, super::Ordering::Relaxed);

        drop(b_guard);
        drop(a_guard);
    }
}

fn main() { cir_trace::init();
    let t1 = cir_trace::spawn("t1#1141", || first::t1(&second::b));
    let t2 = cir_trace::spawn("t2#1195", || second::t2(&first::a));

    t1.join().unwrap();
    t2.join().unwrap();

    let done = if WORK.load(Ordering::Relaxed) == 2 {
        1
    } else {
        0
    };
    println!("DONE done={done}");
 cir_trace::finish();}
