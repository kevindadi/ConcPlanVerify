mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};

use std::thread;

static a: Mutex<()> = Mutex::new_named("a_mutex0", ());

mod other {
    use crate::cir_trace::sync::{Mutex};

    pub static b: Mutex<()> = Mutex::new_named("b_mutex0", ());

    pub fn t2() {
        let a_guard = crate::a.lock().unwrap();
        let b_guard = b.lock().unwrap();

        let mut work = 0;
        work = 1;

        drop(b_guard);
        drop(a_guard);
    }
}

fn t1() {
    let a_guard = a.lock().unwrap();
    let b_guard = other::b.lock().unwrap();

    let mut work = 0;
    work = 1;

    drop(b_guard);
    drop(a_guard);
}

fn main() { crate::cir_trace::init();
    let t1_handle = crate::cir_trace::spawn("t1#575", || t1());
    let t2_handle = crate::cir_trace::spawn("t2#619", || other::t2());

    t1_handle.join().unwrap();
    t2_handle.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
