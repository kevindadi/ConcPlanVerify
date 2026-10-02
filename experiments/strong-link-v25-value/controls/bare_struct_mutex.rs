// UNKNOWN control: the flag lives inside a Mutex<Struct>, not a primitive inner.
// No value event can be attributed, so the goal stays unsupported.
use std::sync::{Arc, Mutex};
use std::thread;

struct S { ready: bool }

fn main() {
    let m = Arc::new(Mutex::new(S { ready: false }));
    let m2 = Arc::clone(&m);
    let h = thread::spawn(move || {
        let mut g = m2.lock().unwrap();
        g.ready = true;
    });
    h.join().unwrap();
    let _ = m.lock().unwrap().ready;
    println!("DONE ready=true");
}
