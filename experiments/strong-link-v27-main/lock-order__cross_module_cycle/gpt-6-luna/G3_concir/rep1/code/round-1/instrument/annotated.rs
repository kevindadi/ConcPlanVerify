mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};


pub mod main {
    use crate::cir_trace::sync::{Mutex};

    pub static a: Mutex<()> = Mutex::new_named("a_mutex0", ());

    pub fn t1() {
        let mut work = 0;
        let a_guard = a.lock().unwrap();
        let b_guard = crate::other::b.lock().unwrap();
        work = 1;
        drop(b_guard);
        drop(a_guard);
        let _ = work;
    }
}

pub mod other {
    use crate::cir_trace::sync::{Mutex};

    pub static b: Mutex<()> = Mutex::new_named("b_mutex0", ());

    pub fn t2() {
        let mut work = 0;
        let a_guard = crate::main::a.lock().unwrap();
        let b_guard = b.lock().unwrap();
        work = 1;
        drop(b_guard);
        drop(a_guard);
        let _ = work;
    }
}

fn main() { cir_trace::init();
    let t1 = cir_trace::spawn("t1#696", main::t1);
    let t2 = cir_trace::spawn("t2#739", other::t2);

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
