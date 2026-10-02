mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

fn t1(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) {
    // main::t1: lock main::a, then other::b, then unlock in reverse order
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn t2(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) {
    // other::t2: lock main::a, then other::b, then unlock in reverse order
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn main() { cir_trace::init();
    // Resources: main::a and other::b
    let a: Arc<Mutex<()>> = Arc::new(Mutex::new_named("a_mutex0#570", ()));
    let b: Arc<Mutex<()>> = Arc::new(Mutex::new_named("b_mutex0#624", ()));

    // main: spawn t1 (h1), spawn t2 (h2), join both
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("t1#759", move || t1(&a1, &b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = cir_trace::spawn("t2#873", move || t2(&a2, &b2));

    h1.join().expect("t1 panicked");
    h2.join().expect("t2 panicked");

    // Terminal line required by the requirements
    println!("DONE done=1");
 cir_trace::finish();}
