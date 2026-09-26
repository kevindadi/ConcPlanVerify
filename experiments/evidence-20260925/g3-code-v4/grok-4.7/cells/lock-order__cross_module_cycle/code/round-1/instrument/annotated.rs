mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};

use std::thread;

#[allow(non_upper_case_globals)]
static a: Mutex<()> = Mutex::new_named("a_mutex0", ());

mod other {
    use crate::cir_trace::sync::{Mutex};

    #[allow(non_upper_case_globals)]
    pub static b: Mutex<()> = Mutex::new_named("b_mutex0", ());

    pub fn t2() {
        let mut work = 0;
        let guard_a = crate::a.lock().unwrap();
        let guard_b = b.lock().unwrap();
        work = 1;
        drop(guard_b);
        drop(guard_a);
        let _ = work;
    }
}

fn t1() {
    let mut work = 0;
    let guard_a = a.lock().unwrap();
    let guard_b = other::b.lock().unwrap();
    work = 1;
    drop(guard_b);
    drop(guard_a);
    let _ = work;
}

fn main() { cir_trace::init();
    let h1 = cir_trace::spawn("h1", t1);
    let h2 = cir_trace::spawn("h2", other::t2);
    h1.join().unwrap();
    h2.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}
