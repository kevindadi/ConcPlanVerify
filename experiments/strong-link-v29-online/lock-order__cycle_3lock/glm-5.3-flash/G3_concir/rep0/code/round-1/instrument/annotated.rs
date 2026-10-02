mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) {
    let _g_a = a.lock().unwrap();
    let _g_b = b.lock().unwrap();
    // critical work while holding both locks
    drop(_g_b);
    drop(_g_a);
}

fn t2(b: &Arc<Mutex<()>>, c: &Arc<Mutex<()>>) {
    let _g_b = b.lock().unwrap();
    let _g_c = c.lock().unwrap();
    // critical work while holding both locks
    drop(_g_c);
    drop(_g_b);
}

fn t3(a: &Arc<Mutex<()>>, c: &Arc<Mutex<()>>) {
    let _g_a = a.lock().unwrap();
    let _g_c = c.lock().unwrap();
    // critical work while holding both locks
    drop(_g_c);
    drop(_g_a);
}

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#678", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#716", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#754", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = crate::cir_trace::spawn("t1#836", move || t1(&a1, &b1));

    let b2 = Arc::clone(&b);
    let c2 = Arc::clone(&c);
    let h2 = crate::cir_trace::spawn("t2#945", move || t2(&b2, &c2));

    let a3 = Arc::clone(&a);
    let c3 = Arc::clone(&c);
    let h3 = crate::cir_trace::spawn("t3#1054", move || t3(&a3, &c3));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
