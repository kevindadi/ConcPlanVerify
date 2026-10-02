mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn x1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();
    drop(_guard_b);
    drop(_guard_a);
}

fn x2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();
    drop(_guard_b);
    drop(_guard_a);
}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let h1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("x1#513", move || x1(a, b))
    };
    let h2 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("x2#639", move || x2(a, b))
    };
    h1.join().unwrap();
    h2.join().unwrap();
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#769", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#807", ()));

    let outer_handle = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("outer#915", move || outer(a, b))
    };
    outer_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
