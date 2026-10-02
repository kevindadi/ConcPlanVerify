mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

mod cir_modules {
    pub mod main {
        use std::sync::{Arc};use crate::cir_trace::sync::{Mutex};
        use std::thread;

        pub fn main(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
            let a1 = Arc::clone(&a);
            let b1 = Arc::clone(&b);
            let t1_handle = cir_trace::spawn("t1#309", move || t1(a1, b1));

            let a2 = Arc::clone(&a);
            let b2 = Arc::clone(&b);
            let t2_handle = cir_trace::spawn("t2#447", move || super::other::t2(a2, b2));

            t1_handle.join().unwrap();
            t2_handle.join().unwrap();
        }

        pub fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
            let guard_a = a.lock().unwrap();
            let guard_b = b.lock().unwrap();
            drop(guard_b);
            drop(guard_a);
        }
    }

    pub mod other {
        use std::sync::{Arc};use crate::cir_trace::sync::{Mutex};

        pub fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
            let guard_a = a.lock().unwrap();
            let guard_b = b.lock().unwrap();
            drop(guard_b);
            drop(guard_a);
        }
    }
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#1124", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#1162", ()));

    cir_modules::main::main(a, b);
    println!("DONE done=1");
 cir_trace::finish();}
