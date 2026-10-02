use std::sync::{Arc, Mutex};
use std::thread;

// Shared resources: a, b (first pair); c, d (second pair).
// Lock ordering rule (R6, R7, R8):
//   - t1 and t2 both acquire a before b (same relative sequence).
//   - t3 and t4 both acquire c before d (same relative sequence).
// The two pairs are independent (R5): no thread ever touches a lock
// from the other pair, so progress in one pair never blocks the other.

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap(); // acquire a first
    let _gb = b.lock().unwrap(); // then b: holds both at once (R4)
    // ... work while holding both locks ...
    // _gb and _ga drop here, releasing both before finishing (R9)
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap(); // acquire a first (same order as t1)
    let _gb = b.lock().unwrap(); // then b
    // ... work while holding both locks ...
}

fn t3(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let _gc = c.lock().unwrap(); // acquire c first
    let _gd = d.lock().unwrap(); // then d: holds both at once (R4)
    // ... work while holding both locks ...
}

fn t4(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let _gc = c.lock().unwrap(); // acquire c first (same order as t3)
    let _gd = d.lock().unwrap(); // then d
    // ... work while holding both locks ...
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));
    let d = Arc::new(Mutex::new(()));

    let mut handles = Vec::new();

    // First pair of workers: t1, t2 share locks a and b.
    {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        handles.push(thread::spawn(move || t1(a, b)));
    }
    {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        handles.push(thread::spawn(move || t2(a, b)));
    }

    // Second pair of workers: t3, t4 share locks c and d.
    {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        handles.push(thread::spawn(move || t3(c, d)));
    }
    {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        handles.push(thread::spawn(move || t4(c, d)));
    }

    // Main thread starts all four workers and waits for all of them (R10).
    for h in handles {
        h.join().unwrap();
    }

    // Printed exactly once, after every worker has finished (R12).
    println!("DONE done=1");
}
