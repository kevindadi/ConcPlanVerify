mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};

use std::thread;

mod other {
    use crate::cir_trace::sync::{Mutex};

    pub static b: Mutex<()> = Mutex::new_named("b_mutex0", ());

    pub fn t2() {
        let _guard_a = crate::main::a.lock().unwrap();
        let _guard_b = b.lock().unwrap();
        drop(_guard_b);
        drop(_guard_a);
    }
}

mod main {
    use crate::cir_trace::sync::{Mutex};
    use std::thread;

    pub static a: Mutex<()> = Mutex::new_named("a_mutex0", ());

    pub fn t1() {
        let _guard_a = a.lock().unwrap();
        let _guard_b = crate::other::b.lock().unwrap();
        drop(_guard_b);
        drop(_guard_a);
    }

    pub fn main() {
        let handle_t1 = cir_trace::spawn("handle_t1", t1);
        let handle_t2 = cir_trace::spawn("handle_t2", crate::other::t2);

        handle_t1.join().unwrap();
        handle_t2.join().unwrap();
    }
}

fn main() { cir_trace::init();
    main::main();
    println!("DONE done=1");
 cir_trace::finish();}
