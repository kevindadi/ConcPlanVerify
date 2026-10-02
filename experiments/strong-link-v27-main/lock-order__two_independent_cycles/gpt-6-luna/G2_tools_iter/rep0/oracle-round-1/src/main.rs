mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
    std::hint::black_box(());
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
    std::hint::black_box(());
}

fn t3(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let _c_guard = c.lock().unwrap();
    let _d_guard = d.lock().unwrap();
    std::hint::black_box(());
}

fn t4(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let _c_guard = c.lock().unwrap();
    let _d_guard = d.lock().unwrap();
    std::hint::black_box(());
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#707", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#745", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#783", ()));
    let d = Arc::new(Mutex::new_named("d_mutex0#821", ()));

    let h1 = cir_trace::spawn("t1#845", {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        move || t1(a, b)
    });
    let h2 = cir_trace::spawn("t2#971", {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        move || t2(a, b)
    });
    let h3 = cir_trace::spawn("t3#1097", {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        move || t3(c, d)
    });
    let h4 = cir_trace::spawn("t4#1223", {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        move || t4(c, d)
    });

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
