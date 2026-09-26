use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(0i32));
    let b = Arc::new(Mutex::new(()));

    let a_outer = Arc::clone(&a);
    let outer_handle = thread::spawn(move || {
        outer(a_outer, b);
    });
    outer_handle.join().unwrap();

    let done = *a.lock().unwrap();
    println!("DONE done={done}");
}

fn outer(a: Arc<Mutex<i32>>, b: Arc<Mutex<()>>) {
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let x1_handle = thread::spawn(move || {
        x1(a1, b1);
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let x2_handle = thread::spawn(move || {
        x2(a2, b2);
    });

    x1_handle.join().unwrap();
    x2_handle.join().unwrap();

    {
        let mut done = a.lock().unwrap();
        *done = 1;
    }
}

fn x1(a: Arc<Mutex<i32>>, b: Arc<Mutex<()>>) {
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    drop(gb);
    drop(ga);
}

fn x2(a: Arc<Mutex<i32>>, b: Arc<Mutex<()>>) {
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    drop(gb);
    drop(ga);
}
