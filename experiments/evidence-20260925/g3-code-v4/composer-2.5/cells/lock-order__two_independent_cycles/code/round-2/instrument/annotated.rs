mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

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

    let t1 = cir_trace::spawn("t1", {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        move || t1(a, b)
    });
    let t2 = cir_trace::spawn("t2", {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        move || t2(a, b)
    });
    let t3 = cir_trace::spawn("t3", {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        move || t3(c, d)
    });
    let t4 = cir_trace::spawn("t4", {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        move || t4(c, d)
    });

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();
    t4.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
