mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{atomic::{AtomicUsize, Ordering};
use std::thread;

mod module_a {
    use super::*;

    /// Resource `a` is owned by this module. Task `t1` depends on resource `b` (module_b).
    pub fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>, order: Arc<Semaphore>, done: Arc<AtomicUsize>) {
        let _order = order.acquire();
        let _a = a.lock().expect("lock a");
        let _b = b.lock().expect("lock b");
        // work while holding both a and b
        drop(_b);
        drop(_a);
        drop(_order);
        done.fetch_add(1, Ordering::SeqCst);
    }
}

mod module_b {
    use super::*;

    /// Resource `b` is owned by this module. Task `t2` depends on resource `a` (module_a).
    pub fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>, order: Arc<Semaphore>, done: Arc<AtomicUsize>) {
        let _order = order.acquire();
        let _a = a.lock().expect("lock a");
        let _b = b.lock().expect("lock b");
        // work while holding both a and b
        drop(_b);
        drop(_a);
        drop(_order);
        done.fetch_add(1, Ordering::SeqCst);
    }
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));
    let order = Semaphore::new_named("order_semaphore0", 1);
    let done = Arc::new(AtomicUsize::new(0));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let order1 = Arc::clone(&order);
    let done1 = Arc::clone(&done);
    let t1_handle = cir_trace::spawn("t1", move || {
        module_a::t1(a1, b1, order1, done1);
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let order2 = Arc::clone(&order);
    let done2 = Arc::clone(&done);
    let t2_handle = cir_trace::spawn("t2", move || {
        module_b::t2(a2, b2, order2, done2);
    });

    t1_handle.join().expect("join t1");
    t2_handle.join().expect("join t2");

    assert_eq!(done.load(Ordering::SeqCst), 2);
    println!("DONE done=1");
 cir_trace::finish();}
