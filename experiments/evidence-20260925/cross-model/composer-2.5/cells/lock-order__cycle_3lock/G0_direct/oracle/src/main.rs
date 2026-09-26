mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>, order: Arc<Semaphore>) {
    let _order = order.acquire();
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    _order.release();
    // critical work (both locks held)
}

fn t2(b: Arc<Mutex<()>>, c: Arc<Mutex<()>>, order: Arc<Semaphore>) {
    let _order = order.acquire();
    let _gb = b.lock().unwrap();
    let _gc = c.lock().unwrap();
    _order.release();
    // critical work (both locks held)
}

fn t3(c: Arc<Mutex<()>>, a: Arc<Mutex<()>>, order: Arc<Semaphore>) {
    let _order = order.acquire();
    let _ga = a.lock().unwrap();
    let _gc = c.lock().unwrap();
    _order.release();
    // critical work (both locks held)
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0", ()));
    let order = Semaphore::new_named("order_semaphore0", 1);

    let h1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let order = Arc::clone(&order);
        cir_trace::spawn("t1", move || t1(a, b, order))
    };
    let h2 = {
        let b = Arc::clone(&b);
        let c = Arc::clone(&c);
        let order = Arc::clone(&order);
        cir_trace::spawn("t2", move || t2(b, c, order))
    };
    let h3 = {
        let c = Arc::clone(&c);
        let a = Arc::clone(&a);
        let order = Arc::clone(&order);
        cir_trace::spawn("t3", move || t3(c, a, order))
    };

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
