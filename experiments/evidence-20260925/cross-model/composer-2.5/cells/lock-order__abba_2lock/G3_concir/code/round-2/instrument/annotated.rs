mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
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

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", AState {
        done_t1: 0,
        done_t2: 0,
    }));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("t1", move || t1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = cir_trace::spawn("t2", move || t2(a2, b2));

    h1.join().unwrap();
    h2.join().unwrap();

    emit_done();
 cir_trace::finish();}
