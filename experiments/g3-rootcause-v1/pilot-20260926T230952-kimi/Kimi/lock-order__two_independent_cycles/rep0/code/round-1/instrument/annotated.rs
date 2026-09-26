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

    let h1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t1", move || t1(a, b))
    };
    let h2 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t2", move || t2(a, b))
    };
    let h3 = {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        cir_trace::spawn("t3", move || t3(c, d))
    };
    let h4 = {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        cir_trace::spawn("t4", move || t4(c, d))
    };

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
