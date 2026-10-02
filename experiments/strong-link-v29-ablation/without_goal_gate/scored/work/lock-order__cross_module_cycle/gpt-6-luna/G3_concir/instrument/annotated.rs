mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>) {
    let a_guard = a.lock().unwrap();
    let b_guard = other::b.lock().unwrap();
    drop(b_guard);
    drop(a_guard);
}

mod other {
    use std::sync::{Arc};use crate::cir_trace::sync::{Mutex};

    pub static b: Mutex<()> = Mutex::new_named("b_mutex0", ());

    pub fn t2(a: Arc<Mutex<()>>) {
        let a_guard = a.lock().unwrap();
        let b_guard = b.lock().unwrap();
        drop(b_guard);
        drop(a_guard);
    }
}

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#501", ()));
    let t1_a = Arc::clone(&a);
    let t2_a = Arc::clone(&a);

    let handle1 = crate::cir_trace::spawn("t1#592", move || t1(t1_a));
    let handle2 = crate::cir_trace::spawn("t2#643", move || other::t2(t2_a));

    handle1.join().unwrap();
    handle2.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
