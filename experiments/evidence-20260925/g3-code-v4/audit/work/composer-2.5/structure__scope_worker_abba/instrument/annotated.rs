mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};


fn w1(a: &Mutex<()>, b: &Mutex<()>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
}

fn w2(a: &Mutex<()>, b: &Mutex<()>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
}

fn main() { cir_trace::init();
    let a = Mutex::new_named("a_mutex0", ());
    let b = Mutex::new_named("b_mutex0", ());

    std::thread::scope(|s| {
        s.spawn(|| w1(&a, &b));
        s.spawn(|| w2(&a, &b));
    });

    println!("DONE done=1");
 cir_trace::finish();}
