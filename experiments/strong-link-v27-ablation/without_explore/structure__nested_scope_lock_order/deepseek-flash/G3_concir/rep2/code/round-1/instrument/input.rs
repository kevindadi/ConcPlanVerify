use std::sync::Mutex;
use std::thread;

#[allow(non_upper_case_globals)]
static a: Mutex<()> = Mutex::new(());

#[allow(non_upper_case_globals)]
static b: Mutex<()> = Mutex::new(());

fn x1() {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn x2() {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn outer() {
    let h_x1 = thread::spawn(x1);
    let h_x2 = thread::spawn(x2);
    h_x1.join().unwrap();
    h_x2.join().unwrap();
}

fn main() {
    let h_outer = thread::spawn(outer);
    h_outer.join().unwrap();
    println!("DONE done=1");
}
