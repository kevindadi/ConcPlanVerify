use std::sync::{Arc, Mutex};
use std::thread;

struct AState {
    done_t1: i32,
    done_t2: i32,
}

fn t1(a: Arc<Mutex<AState>>, b: Arc<Mutex<()>>) {
    let mut ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    ga.done_t1 = 1;
    drop(_gb);
    drop(ga);
}

fn t2(a: Arc<Mutex<AState>>, b: Arc<Mutex<()>>) {
    let mut ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    ga.done_t2 = 1;
    drop(_gb);
    drop(ga);
}

fn emit_done() {
    println!("DONE t1=1 t2=1");
}

fn main() {
    let a = Arc::new(Mutex::new(AState {
        done_t1: 0,
        done_t2: 0,
    }));
    let b = Arc::new(Mutex::new(()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = thread::spawn(move || t1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = thread::spawn(move || t2(a2, b2));

    h1.join().unwrap();
    h2.join().unwrap();

    emit_done();
}
