mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap();
    let t: i32 = *c;
    *c = t + 1;
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap();
    let t: i32 = *c;
    *c = t + 1;
}

fn main() { cir_trace::init();
    let m: Arc<Mutex<i32>> = Arc::new(Mutex::new_named("res_mutex0", 0));

    let h1 = cir_trace::spawn("h1", {
        let m = Arc::clone(&m);
        move || w1(m)
    });
    let h2 = cir_trace::spawn("h2", {
        let m = Arc::clone(&m);
        move || w2(m)
    });

    h1.join().unwrap();
    h2.join().unwrap();

    let done: i32 = 1;
    println!("DONE done={}", done);
 cir_trace::finish();}
