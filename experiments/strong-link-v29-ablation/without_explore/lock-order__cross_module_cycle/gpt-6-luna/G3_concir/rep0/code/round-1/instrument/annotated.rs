mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

mod module1 {
    use std::sync::{Arc};use crate::cir_trace::sync::{Mutex};
    use std::thread;

    pub fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
        let a_guard = a.lock().unwrap();
        let b_guard = b.lock().unwrap();

        let mut work = 0;
        work = work + 1;

        drop(b_guard);
        drop(a_guard);
    }

    pub fn main() {
        let a = Arc::new(Mutex::new_named("a_mutex0#410", ()));
        let b = Arc::new(Mutex::new_named("b_mutex0#452", ()));

        let a1 = Arc::clone(&a);
        let b1 = Arc::clone(&b);
        let t1_handle = crate::cir_trace::spawn("t1#553", move || t1(a1, b1));

        let a2 = Arc::clone(&a);
        let b2 = Arc::clone(&b);
        let t2_handle = crate::cir_trace::spawn("t2#679", move || crate::module2::t2(a2, b2));

        t1_handle.join().unwrap();
        t2_handle.join().unwrap();

        println!("DONE done=1");
    }
}

mod module2 {
    use std::sync::{Arc};use crate::cir_trace::sync::{Mutex};

    pub fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
        let a_guard = a.lock().unwrap();
        let b_guard = b.lock().unwrap();

        let mut work = 0;
        work = work + 1;

        drop(b_guard);
        drop(a_guard);
    }
}

fn main() { crate::cir_trace::init();
    module1::main();
 crate::cir_trace::finish();}
