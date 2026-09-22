mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    a: Mutex<()>,
    b: Mutex<()>,
    done1: Mutex<bool>,
    done2: Mutex<bool>,
}

fn t1(shared: Arc<Shared>) {
    {
        let _ga = shared.a.lock().unwrap();
        let _gb = shared.b.lock().unwrap();
        {
            let mut d = shared.done1.lock().unwrap();
            *d = true;
        }
    }
}

fn t2(shared: Arc<Shared>) {
    {
        let _ga = shared.a.lock().unwrap();
        let _gb = shared.b.lock().unwrap();
        {
            let mut d = shared.done2.lock().unwrap();
            *d = true;
        }
    }
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        a: Mutex::new_named("shared_mutex0", ()),
        b: Mutex::new_named("shared_mutex1", ()),
        done1: Mutex::new_named("shared_mutex2", false),
        done2: Mutex::new_named("shared_mutex3", false),
    });

    let s1 = Arc::clone(&shared);
    let h1 = cir_trace::spawn("h1", move || t1(s1));

    let s2 = Arc::clone(&shared);
    let h2 = cir_trace::spawn("h2", move || t2(s2));

    h1.join().unwrap();
    h2.join().unwrap();

    let d1 = *shared.done1.lock().unwrap();
    let d2 = *shared.done2.lock().unwrap();

    println!("DONE t1={} t2={}", d1 as u8, d2 as u8);
 cir_trace::finish();}
