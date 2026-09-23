mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Locks {
    a: Mutex<()>,
    b: Mutex<()>,
}

fn t1(locks: Arc<Locks>) {
    {
        let _ga = locks.a.lock().unwrap();
        let _gb = locks.b.lock().unwrap();
    }
}

fn t2(locks: Arc<Locks>) {
    {
        let _ga = locks.a.lock().unwrap();
        let _gb = locks.b.lock().unwrap();
    }
}

fn main() { cir_trace::init();
    let locks = Arc::new(Locks {
        a: Mutex::new_named("locks_mutex0", ()),
        b: Mutex::new_named("locks_mutex1", ()),
    });

    let l1 = Arc::clone(&locks);
    let h1 = cir_trace::spawn("t1", move || t1(l1));

    let l2 = Arc::clone(&locks);
    let h2 = cir_trace::spawn("t2", move || t2(l2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
