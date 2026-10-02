mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

fn main() { cir_trace::init();
    let m: Arc<Mutex<i32>> = Arc::new(Mutex::new_named("res_mutex0#87", 0));

    let m1 = Arc::clone(&m);
    let h1 = cir_trace::spawn("h1#139", move || {
        let mut guard = m1.lock().unwrap();
        *guard = *guard + 1;
        drop(guard);
    });

    let m2 = Arc::clone(&m);
    let h2 = cir_trace::spawn("h2#313", move || {
        let mut guard = m2.lock().unwrap();
        *guard = *guard + 1;
        drop(guard);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
