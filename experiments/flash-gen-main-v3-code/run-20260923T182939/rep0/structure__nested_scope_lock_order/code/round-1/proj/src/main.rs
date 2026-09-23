mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Locks {
    a: Mutex<()>,
    b: Mutex<()>,
}

fn x1(locks: Arc<Locks>) {
    {
        let _ga = locks.a.lock().unwrap();
        let _gb = locks.b.lock().unwrap();
    }
}

fn x2(locks: Arc<Locks>) {
    {
        let _ga = locks.a.lock().unwrap();
        let _gb = locks.b.lock().unwrap();
    }
}

fn outer(locks: Arc<Locks>) {
    let l1 = Arc::clone(&locks);
    let l2 = Arc::clone(&locks);
    let h1 = cir_trace::spawn("x1", move || x1(l1));
    let h2 = cir_trace::spawn("x2", move || x2(l2));
    h1.join().unwrap();
    h2.join().unwrap();
}

fn main() { cir_trace::init();
    let locks = Arc::new(Locks {
        a: Mutex::new_named("locks_mutex0", ()),
        b: Mutex::new_named("locks_mutex1", ()),
    });
    let l = Arc::clone(&locks);
    let h_outer = cir_trace::spawn("outer", move || outer(l));
    h_outer.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}
