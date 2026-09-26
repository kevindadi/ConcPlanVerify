use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

static C: AtomicI32 = AtomicI32::new(0);

fn print() {}

fn w1() {
    loop {
        let v = C.load(Ordering::SeqCst);
        let nv = v + 1;
        let ok = match C.compare_exchange(v, nv, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(prev) => prev,
            Err(actual) => actual,
        };
        if ok == v {
            break;
        }
    }
}

fn w2() {
    loop {
        let v = C.load(Ordering::SeqCst);
        let nv = v + 1;
        let ok = match C.compare_exchange(v, nv, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(prev) => prev,
            Err(actual) => actual,
        };
        if ok == v {
            break;
        }
    }
}

fn main() {
    let h1: JoinHandle<()> = thread::spawn(w1);
    let h2: JoinHandle<()> = thread::spawn(w2);
    h1.join().unwrap();
    h2.join().unwrap();
    print();
    println!("DONE done=1");
}
