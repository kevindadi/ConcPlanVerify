use std::sync::Mutex;
use std::thread;

static a: Mutex<()> = Mutex::new(());
static b: Mutex<()> = Mutex::new(());

fn x1() {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    drop(b_guard);
    drop(a_guard);
}

fn x2() {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    drop(b_guard);
    drop(a_guard);
}

fn outer() {
    thread::scope(|s| {
        s.spawn(x1);
        s.spawn(x2);
    });
}

fn main() {
    let outer_handle = thread::spawn(outer);
    outer_handle.join().unwrap();
    println!("DONE done=1");
}
