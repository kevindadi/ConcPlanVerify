mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    let mut tmp: i32 = 0;
    tmp = 1;
    let _ = tmp;
    drop(gb);
    drop(ga);
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    let mut tmp: i32 = 0;
    tmp = 1;
    let _ = tmp;
    drop(gb);
    drop(ga);
}

fn t3(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let gc = c.lock().unwrap();
    let gd = d.lock().unwrap();
    let mut tmp: i32 = 0;
    tmp = 1;
    let _ = tmp;
    drop(gd);
    drop(gc);
}

fn t4(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let gc = c.lock().unwrap();
    let gd = d.lock().unwrap();
    let mut tmp: i32 = 0;
    tmp = 1;
    let _ = tmp;
    drop(gd);
    drop(gc);
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#875", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#913", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#951", ()));
    let d = Arc::new(Mutex::new_named("d_mutex0#989", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("t1#1071", move || t1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = cir_trace::spawn("t2#1178", move || t2(a2, b2));

    let c3 = Arc::clone(&c);
    let d3 = Arc::clone(&d);
    let h3 = cir_trace::spawn("t3#1285", move || t3(c3, d3));

    let c4 = Arc::clone(&c);
    let d4 = Arc::clone(&d);
    let h4 = cir_trace::spawn("t4#1392", move || t4(c4, d4));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
