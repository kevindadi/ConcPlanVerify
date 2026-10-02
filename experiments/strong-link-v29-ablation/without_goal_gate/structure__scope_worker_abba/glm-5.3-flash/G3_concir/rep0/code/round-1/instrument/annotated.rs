mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

fn w1(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn w2(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#364", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#402", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = crate::cir_trace::spawn("w1#484", move || w1(&a1, &b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = crate::cir_trace::spawn("w2#598", move || w2(&a2, &b2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
