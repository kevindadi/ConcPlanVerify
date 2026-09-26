mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn println() {
    println!("DONE done=1");
}

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    drop(gb);
    drop(ga);
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    drop(gb);
    drop(ga);
}

fn t3(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let gc = c.lock().unwrap();
    let gd = d.lock().unwrap();
    drop(gd);
    drop(gc);
}

fn t4(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let gc = c.lock().unwrap();
    let gd = d.lock().unwrap();
    drop(gd);
    drop(gc);
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0", ()));
    let d = Arc::new(Mutex::new_named("d_mutex0", ()));

    let t1_handle = cir_trace::spawn("t1_handle", {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        move || t1(a, b)
    });
    let t2_handle = cir_trace::spawn("t2_handle", {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        move || t2(a, b)
    });
    let t3_handle = cir_trace::spawn("t3_handle", {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        move || t3(c, d)
    });
    let t4_handle = cir_trace::spawn("t4_handle", {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        move || t4(c, d)
    });

    t1_handle.join().unwrap();
    t2_handle.join().unwrap();
    t3_handle.join().unwrap();
    t4_handle.join().unwrap();

    println();
 cir_trace::finish();}
