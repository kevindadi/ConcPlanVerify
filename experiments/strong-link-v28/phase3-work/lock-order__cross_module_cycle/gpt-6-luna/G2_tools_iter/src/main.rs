mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
mod first {
    use crate::cir_trace::sync::{Mutex};

    // The first module owns resource a.
    #[allow(non_upper_case_globals)]
    pub static a: Mutex<()> = Mutex::new_named("a_mutex0", ());

    pub fn t1() {
        // t1 also depends on resource b, owned by the second module.
        let guard_a = a.lock().unwrap();
        let guard_b = crate::second::b.lock().unwrap();

        // Work while holding both resources.
        drop(guard_b);
        drop(guard_a);
    }
}

mod second {
    use crate::cir_trace::sync::{Mutex};

    // The second module owns resource b.
    #[allow(non_upper_case_globals)]
    pub static b: Mutex<()> = Mutex::new_named("b_mutex0", ());

    pub fn t2() {
        // t2 also depends on resource a, owned by the first module.
        // Both tasks acquire resources in the same order: a, then b.
        let guard_a = crate::first::a.lock().unwrap();
        let guard_b = b.lock().unwrap();

        // Work while holding both resources.
        drop(guard_b);
        drop(guard_a);
    }
}

fn main() { cir_trace::init();
    let t1 = cir_trace::spawn("t1#997", first::t1);
    let t2 = cir_trace::spawn("t2#1041", second::t2);

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
