use std::sync::{Arc, Mutex};
use std::thread;

fn x1(a: &Mutex<()>, b: &Mutex<()>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn x2(a: &Mutex<()>, b: &Mutex<()>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    thread::scope(|scope| {
        let a1 = &a;
        let b1 = &b;
        let a2 = &a;
        let b2 = &b;

        let x1_handle = scope.spawn(move || x1(a1, b1));
        let x2_handle = scope.spawn(move || x2(a2, b2));

        x1_handle.join().unwrap();
        x2_handle.join().unwrap();
    });
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    thread::scope(|scope| {
        let outer_handle = scope.spawn(|| outer(a, b));
        outer_handle.join().unwrap();
    });

    println!("DONE done=1");
}
