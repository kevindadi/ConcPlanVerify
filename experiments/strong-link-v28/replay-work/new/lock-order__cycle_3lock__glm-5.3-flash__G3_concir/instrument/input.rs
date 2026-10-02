#![allow(non_upper_case_globals)]

use std::sync::Mutex;

static a: Mutex<()> = Mutex::new(());
static b: Mutex<()> = Mutex::new(());
static c: Mutex<()> = Mutex::new(());

fn t1() {
    let mut work: i64 = 0;
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    work = work + 1;
    drop(_gb);
    drop(_ga);
}

fn t2() {
    let mut work: i64 = 0;
    let _gb = b.lock().unwrap();
    let _gc = c.lock().unwrap();
    work = work + 1;
    drop(_gc);
    drop(_gb);
}

fn t3() {
    let mut work: i64 = 0;
    let _ga = a.lock().unwrap();
    let _gc = c.lock().unwrap();
    work = work + 1;
    drop(_gc);
    drop(_ga);
}

fn main() {
    let h1 = std::thread::spawn(|| t1());
    let h2 = std::thread::spawn(|| t2());
    let h3 = std::thread::spawn(|| t3());
    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    println!("DONE done=1");
}
