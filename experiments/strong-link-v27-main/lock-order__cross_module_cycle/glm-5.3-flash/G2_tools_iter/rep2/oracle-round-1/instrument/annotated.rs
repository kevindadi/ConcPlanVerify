mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::thread;

// ---------------------------------------------------------------------
// Module 1: owns shared resource `a` (R1) and runs task t1 (R2).
// ---------------------------------------------------------------------
mod m1 {
    use crate::cir_trace::sync::{Mutex};

    // Resource `a`, owned by this module.
    pub static A: Mutex<()> = Mutex::new_named("A_mutex0", ());

    pub fn t1() {
        // R3: t1 declares its dependency on the resource owned by the
        // OTHER module (m2 owns `b`).
        use crate::m2::B;

        // R5: both tasks acquire locks in the same global order
        // (a, then b), so a circular wait is impossible.
        let ga = crate::m1::A.lock().expect("lock a");
        let gb = B.lock().expect("lock b");

        // R4: work is performed while holding BOTH resources.
        let mut x: u64 = 0;
        for i in 0..1000u64 {
            x = x.wrapping_add(i);
        }
        std::hint::black_box(x);

        // R6: release each resource before finishing.
        drop(gb);
        drop(ga);
    }
}

// ---------------------------------------------------------------------
// Module 2: owns shared resource `b` (R1) and runs task t2 (R2).
// ---------------------------------------------------------------------
mod m2 {
    use crate::cir_trace::sync::{Mutex};

    // Resource `b`, owned by this module.
    pub static B: Mutex<()> = Mutex::new_named("B_mutex0", ());

    pub fn t2() {
        // R3: t2 declares its dependency on the resource owned by the
        // OTHER module (m1 owns `a`).
        use crate::m1::A;

        // R5: identical global lock order (a, then b).
        let ga = A.lock().expect("lock a");
        let gb = crate::m2::B.lock().expect("lock b");

        // R4: work is performed while holding BOTH resources.
        let mut x: u64 = 1;
        for i in 0..1000u64 {
            x = x.wrapping_mul(i).wrapping_add(1);
        }
        std::hint::black_box(x);

        // R6: release each resource before finishing.
        drop(gb);
        drop(ga);
    }
}

fn main() { cir_trace::init();
    // R7: the starting thread launches both tasks and finishes only
    // after both have finished (join blocks until completion).
    let h1 = cir_trace::spawn("h1#2157", m1::t1);
    let h2 = cir_trace::spawn("h2#2193", m2::t2);

    // R8: both tasks run in different modules and both are joined here.
    h1.join().expect("t1 panicked");
    h2.join().expect("t2 panicked");

    // R10: exactly one line of output, then exit.
    println!("DONE done=1");
 cir_trace::finish();}
