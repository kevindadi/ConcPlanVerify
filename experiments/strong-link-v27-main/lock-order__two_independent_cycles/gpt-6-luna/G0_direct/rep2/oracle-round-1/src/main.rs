mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a = a.lock().unwrap();
    let _b = b.lock().unwrap();
    let _work = 1 + 1;
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a = a.lock().unwrap();
    let _b = b.lock().unwrap();
    let _work = 1 + 1;
}

fn t3(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let _c = c.lock().unwrap();
    let _d = d.lock().unwrap();
    let _work = 1 + 1;
}

fn t4(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let _c = c.lock().unwrap();
    let _d = d.lock().unwrap();
    let _work = 1 + 1;
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#631", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#669", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#707", ()));
    let d = Arc::new(Mutex::new_named("d_mutex0#745", ()));

    let handles = vec![
        {
            let (a, b) = (Arc::clone(&a), Arc::clone(&b));
            thread::spawn(move || t1(a, b))
        },
        {
            let (a, b) = (Arc::clone(&a), Arc::clone(&b));
            thread::spawn(move || t2(a, b))
        },
        {
            let (c, d) = (Arc::clone(&c), Arc::clone(&d));
            thread::spawn(move || t3(c, d))
        },
        {
            let (c, d) = (Arc::clone(&c), Arc::clone(&d));
            thread::spawn(move || t4(c, d))
        },
    ];

    for handle in handles {
        handle.join().unwrap();
    }

    let done = 1;
    println!("DONE done={done}");
 cir_trace::finish();}
