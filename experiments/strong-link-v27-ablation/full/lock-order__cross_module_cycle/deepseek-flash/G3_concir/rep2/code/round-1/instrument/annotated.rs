mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::thread;

#[allow(non_upper_case_globals)]
pub mod main {
    use crate::cir_trace::sync::{Mutex};

    pub static a: Mutex<()> = Mutex::new_named("a_mutex0", ());

    pub fn t1() {
        // Declare dependency on the other module's resource.
        let guard_a = a.lock().unwrap();
        let guard_b = crate::other::b.lock().unwrap();

        let mut work: i32 = 0;
        work = 1;
        let _ = work;

        // Release each resource before finishing.
        drop(guard_b);
        drop(guard_a);
    }
}

#[allow(non_upper_case_globals)]
pub mod other {
    use crate::cir_trace::sync::{Mutex};

    pub static b: Mutex<()> = Mutex::new_named("b_mutex0", ());

    pub fn t2() {
        // Declare dependency on the other module's resource.
        let guard_a = crate::main::a.lock().unwrap();
        let guard_b = b.lock().unwrap();

        let mut work: i32 = 0;
        work = 1;
        let _ = work;

        // Release each resource before finishing.
        drop(guard_b);
        drop(guard_a);
    }
}

fn main() { cir_trace::init();
    let h1 = cir_trace::spawn("h1#997", crate::main::t1);
    let h2 = cir_trace::spawn("h2#1042", crate::other::t2);

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
