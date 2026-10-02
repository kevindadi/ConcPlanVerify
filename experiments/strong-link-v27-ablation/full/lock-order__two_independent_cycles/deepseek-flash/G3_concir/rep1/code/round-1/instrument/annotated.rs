mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: &Mutex<()>, b: &Mutex<()>) {
    let mut w: i32 = 0;
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    w = 1;
    let _ = w;
    drop(_gb);
    drop(_ga);
}

fn t2(a: &Mutex<()>, b: &Mutex<()>) {
    let mut w: i32 = 0;
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    w = 1;
    let _ = w;
    drop(_gb);
    drop(_ga);
}

fn t3(c: &Mutex<()>, d: &Mutex<()>) {
    let mut w: i32 = 0;
    let _gc = c.lock().unwrap();
    let _gd = d.lock().unwrap();
    w = 1;
    let _ = w;
    drop(_gd);
    drop(_gc);
}

fn t4(c: &Mutex<()>, d: &Mutex<()>) {
    let mut w: i32 = 0;
    let _gc = c.lock().unwrap();
    let _gd = d.lock().unwrap();
    w = 1;
    let _ = w;
    drop(_gd);
    drop(_gc);
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#835", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#873", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#911", ()));
    let d = Arc::new(Mutex::new_named("d_mutex0#949", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("t1#1031", move || {
        t1(&*a1, &*b1);
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = cir_trace::spawn("t2#1159", move || {
        t2(&*a2, &*b2);
    });

    let c3 = Arc::clone(&c);
    let d3 = Arc::clone(&d);
    let h3 = cir_trace::spawn("t3#1287", move || {
        t3(&*c3, &*d3);
    });

    let c4 = Arc::clone(&c);
    let d4 = Arc::clone(&d);
    let h4 = cir_trace::spawn("t4#1415", move || {
        t4(&*c4, &*d4);
    });

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
