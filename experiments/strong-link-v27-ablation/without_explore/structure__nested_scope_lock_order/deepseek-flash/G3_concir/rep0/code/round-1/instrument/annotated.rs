mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#87", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#125", ()));

    let h_outer = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("outer#228", move || outer(a, b))
    };

    h_outer.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let h_x1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("x1#470", move || x1(a, b))
    };

    let h_x2 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("x2#599", move || x2(a, b))
    };

    h_x1.join().unwrap();
    h_x2.join().unwrap();
}

fn x1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();

    drop(gb);
    drop(ga);
}

fn x2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();

    drop(gb);
    drop(ga);
}
