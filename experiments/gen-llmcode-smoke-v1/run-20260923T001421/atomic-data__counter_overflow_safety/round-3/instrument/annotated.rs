mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    c: i32,
    done: bool,
}

fn main() { cir_trace::init();
    let shared = Arc::new(Mutex::new_named("shared_mutex0", Shared { c: 0, done: false }));

    // main spawns w1 (the supervisor worker)
    let shared_w1 = Arc::clone(&shared);
    let sup = cir_trace::spawn("sup", move || {
        w1(shared_w1);
    });

    // main joins the supervisor
    sup.join().unwrap();

    // terminal status line
    let done_val = {
        let g = shared.lock().unwrap();
        g.done
    };
    println!("DONE done={}", if done_val { 1 } else { 0 });
 cir_trace::finish();}

fn w1(shared: Arc<Mutex<Shared>>) {
    // spawn two workers
    let shared_a = Arc::clone(&shared);
    let w1_handle = cir_trace::spawn("w1_handle", move || {
        w2(shared_a);
    });

    let shared_b = Arc::clone(&shared);
    let w2_handle = cir_trace::spawn("w2_handle", move || {
        w2(shared_b);
    });

    // join both workers
    w1_handle.join().unwrap();
    w2_handle.join().unwrap();

    // set done under the lock
    {
        let mut g = shared.lock().unwrap();
        g.done = true;
    }
}

fn w2(shared: Arc<Mutex<Shared>>) {
    let mut g = shared.lock().unwrap();
    let cur = g.c;
    if cur < 1 {
        g.c = cur + 1;
    }
    drop(g);
}
