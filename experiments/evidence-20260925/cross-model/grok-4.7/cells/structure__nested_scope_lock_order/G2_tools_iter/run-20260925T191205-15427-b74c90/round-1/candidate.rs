use std::sync::{Arc, Mutex};
use std::thread;

use concir_sync::Semaphore;

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let a_outer = Arc::clone(&a);
    let b_outer = Arc::clone(&b);

    let outer = thread::spawn(move || {
        outer(a_outer, b_outer);
    });

    outer.join().unwrap();
    println!("DONE done=1");
}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let gate = Semaphore::new(2);

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let g1 = Arc::clone(&gate);

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let g2 = Arc::clone(&gate);

    let x1 = thread::spawn(move || {
        x1(g1, a1, b1);
    });
    let x2 = thread::spawn(move || {
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
