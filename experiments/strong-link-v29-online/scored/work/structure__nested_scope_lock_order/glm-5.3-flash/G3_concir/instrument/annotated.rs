mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

fn x1(a: &Mutex<()>, b: &Mutex<()>) {
    {
        let _guard_a = a.lock().unwrap();
        let _guard_b = b.lock().unwrap();
        // holds both a and b
    } // unlock b, then unlock a (drop order)
}

fn x2(a: &Mutex<()>, b: &Mutex<()>) {
    {
        let _guard_a = a.lock().unwrap();
        let _guard_b = b.lock().unwrap();
        // holds both a and b
    } // unlock b, then unlock a (drop order)
}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let h1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        crate::cir_trace::spawn("x1#580", move || {
            x1(&a, &b);
        })
    };
    let h2 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        crate::cir_trace::spawn("x2#738", move || {
            x2(&a, &b);
        })
    };
    h1.join().unwrap();
    h2.join().unwrap();
}

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#900", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#938", ()));

    let handle = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        crate::cir_trace::spawn("outer#1040", move || {
            outer(a, b);
        })
    };

    handle.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
