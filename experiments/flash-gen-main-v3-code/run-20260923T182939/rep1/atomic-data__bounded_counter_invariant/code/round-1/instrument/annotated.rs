mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
mod concir_sync;

use std::sync::{Arc};
use std::thread;

struct Shared {
    c: i32,
}

fn w1(m: Arc<Mutex<Shared>>) {
    {
        let mut guard = m.lock().unwrap();
        let tmp = guard.c;
        let tmp2 = tmp + 1;
        guard.c = tmp2;
    }
}

fn w2(m: Arc<Mutex<Shared>>) {
    {
        let mut guard = m.lock().unwrap();
        let tmp = guard.c;
        let tmp2 = tmp + 1;
        guard.c = tmp2;
    }
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", Shared { c: 0 }));

    let m1 = Arc::clone(&m);
    let m2 = Arc::clone(&m);

    let h1 = cir_trace::spawn("w1", move || w1(m1));
    let h2 = cir_trace::spawn("w2", move || w2(m2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
