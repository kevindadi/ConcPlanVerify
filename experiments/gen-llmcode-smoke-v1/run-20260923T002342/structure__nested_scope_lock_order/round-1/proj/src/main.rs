mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));

    let a_outer = Arc::clone(&a);
    let b_outer = Arc::clone(&b);

    let h_outer = cir_trace::spawn("h_outer", move || {
        outer(a_outer, b_outer);
    });

    h_outer.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);

    let h_x1 = cir_trace::spawn("h_x1", move || {
        x1(a1, b1);
    });
    let h_x2 = cir_trace::spawn("h_x2", move || {
        x2(a2, b2);
    });

    h_x1.join().unwrap();
    h_x2.join().unwrap();
}

fn x1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    {
        let _ga = a.lock().unwrap();
        let _gb = b.lock().unwrap();
    }
}

fn x2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    {
        let _ga = a.lock().unwrap();
        let _gb = b.lock().unwrap();
    }
}
