mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    a: Mutex<()>,
    b: Mutex<()>,
}

fn t1(shared: Arc<Shared>) {
    let _gb = shared.b.lock().unwrap();
    let _ga = shared.a.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn t2(shared: Arc<Shared>) {
    let _ga = shared.a.lock().unwrap();
    let _gb = shared.b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        a: Mutex::new_named("shared_mutex0", ()),
        b: Mutex::new_named("shared_mutex1", ()),
    });

    let s1 = Arc::clone(&shared);
    let h1 = cir_trace::spawn("t1", move || t1(s1));

    let s2 = Arc::clone(&shared);
    let h2 = cir_trace::spawn("t2", move || t2(s2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
