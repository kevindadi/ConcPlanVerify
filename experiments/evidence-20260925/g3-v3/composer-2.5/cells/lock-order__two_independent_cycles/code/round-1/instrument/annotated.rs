mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread::{self, JoinHandle};

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn t3(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let _gc = c.lock().unwrap();
    let _gd = d.lock().unwrap();
    drop(_gd);
    drop(_gc);
}

fn t4(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let _gc = c.lock().unwrap();
    let _gd = d.lock().unwrap();
    drop(_gd);
    drop(_gc);
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0", ()));
    let d = Arc::new(Mutex::new_named("d_mutex0", ()));

    let workers: [JoinHandle<()>; 4] = [
        cir_trace::spawn("spawn0", {
            let a = Arc::clone(&a);
            let b = Arc::clone(&b);
            move || t1(a, b)
        }),
        cir_trace::spawn("spawn1", {
            let a = Arc::clone(&a);
            let b = Arc::clone(&b);
            move || t2(a, b)
        }),
        cir_trace::spawn("spawn2", {
            let c = Arc::clone(&c);
            let d = Arc::clone(&d);
            move || t3(c, d)
        }),
        cir_trace::spawn("spawn3", {
            let c = Arc::clone(&c);
            let d = Arc::clone(&d);
            move || t4(c, d)
        }),
    ];

    for h in workers {
        h.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
