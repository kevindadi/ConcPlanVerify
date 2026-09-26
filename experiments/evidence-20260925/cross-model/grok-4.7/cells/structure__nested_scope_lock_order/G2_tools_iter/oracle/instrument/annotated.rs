mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

use concir_sync::Semaphore;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));

    let a_outer = Arc::clone(&a);
    let b_outer = Arc::clone(&b);

    let outer = cir_trace::spawn("outer", move || {
        outer(a_outer, b_outer);
    });

    outer.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let gate = Semaphore::new_named("gate_semaphore0", 2);

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let g1 = Arc::clone(&gate);

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let g2 = Arc::clone(&gate);

    let x1 = cir_trace::spawn("x1", move || {
        x1(g1, a1, b1);
    });
    let x2 = cir_trace::spawn("x2", move || {
        x2(g2, a2, b2);
    });

    x1.join().unwrap();
    x2.join().unwrap();
}

fn x1(gate: Arc<Semaphore>, a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _permit = gate.acquire();
    let _hold_a = a.lock().unwrap();
    let _hold_b = b.lock().unwrap();
}

fn x2(gate: Arc<Semaphore>, a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _permit = gate.acquire();
    let _hold_a = a.lock().unwrap();
    let _hold_b = b.lock().unwrap();
}
