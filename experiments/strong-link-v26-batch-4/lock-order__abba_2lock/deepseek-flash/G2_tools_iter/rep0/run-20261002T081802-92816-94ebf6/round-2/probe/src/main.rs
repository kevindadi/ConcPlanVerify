#![allow(non_upper_case_globals)]

use std::sync::Mutex;
use std::thread;

static a: Mutex<()> = Mutex::new(());
static b: Mutex<()> = Mutex::new(());

fn t1() {
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();

    // Critical work while holding both locks.
}

fn t2() {
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();

    // Critical work while holding both locks.
}

fn main() {
    let t1_handle = thread::spawn(t1);
    let t2_handle = thread::spawn(t2);

    t1_handle.join().unwrap();
    t2_handle.join().unwrap();

    println!("DONE t1=1 t2=1");
}
