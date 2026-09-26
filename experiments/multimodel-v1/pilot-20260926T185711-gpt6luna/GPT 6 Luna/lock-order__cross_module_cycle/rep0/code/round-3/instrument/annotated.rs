mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};

use std::thread;

pub static a: Mutex<()> = Mutex::new_named("a_mutex0", ());

mod other {
    use crate::cir_trace::sync::{Mutex};

    pub static b: Mutex<()> = Mutex::new_named("b_mutex0", ());

    pub fn t2() {
        let a_guard = crate::a.lock().unwrap();
        let b_guard = b.lock().unwrap();
        drop(b_guard);
        drop(a_guard);
    }
}

fn t1() {
    let a_guard = a.lock().unwrap();
    let b_guard = other::b.lock().unwrap();
    drop(b_guard);
    drop(a_guard);
}

fn main() { cir_trace::init();
    let t1_handle = cir_trace::spawn("t1_handle", t1);
    let t2_handle = cir_trace::spawn("t2_handle", other::t2);

    t1_handle.join().unwrap();
    t2_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
