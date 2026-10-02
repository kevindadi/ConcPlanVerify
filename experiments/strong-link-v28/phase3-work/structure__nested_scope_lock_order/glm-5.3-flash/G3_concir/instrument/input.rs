use std::sync::{Arc, Mutex};
use std::thread;

struct Shared {
    done: i32,
}

fn x1(a: &Arc<Mutex<Shared>>, b: &Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn x2(a: &Arc<Mutex<Shared>>, b: &Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn outer(a: Arc<Mutex<Shared>>, b: Arc<Mutex<()>>) {
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h_x1 = thread::spawn(move || {
        x1(&a1, &b1);
    });
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h_x2 = thread::spawn(move || {
        x2(&a2, &b2);
    });
    h_x1.join().unwrap();
    h_x2.join().unwrap();
    let mut guard = a.lock().unwrap();
    guard.done = 1;
    drop(guard);
}

fn main() {
    let a = Arc::new(Mutex::new(Shared { done: 0 }));
    let b = Arc::new(Mutex::new(()));

    let a_outer = Arc::clone(&a);
    let b_outer = Arc::clone(&b);
    let h_outer = thread::spawn(move || {
        outer(a_outer, b_outer);
    });

    h_outer.join().unwrap();

    let guard = a.lock().unwrap();
    let done = guard.done;
    drop(guard);
    println!("DONE done={}", done);
}
