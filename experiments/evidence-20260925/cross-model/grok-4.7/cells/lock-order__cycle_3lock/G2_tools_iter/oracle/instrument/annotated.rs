mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn t1(a: Arc<Semaphore>, b: Arc<Semaphore>) {
    let _pa = a.acquire();
    let _pb = b.acquire();
}

fn t2(b: Arc<Semaphore>, c: Arc<Semaphore>) {
    let _pb = b.acquire();
    let _pc = c.acquire();
}

fn t3(a: Arc<Semaphore>, c: Arc<Semaphore>) {
    let _pa = a.acquire();
    let _pc = c.acquire();
}

fn main() { cir_trace::init();
    let a = Semaphore::new_named("a_semaphore0", 1);
    let b = Semaphore::new_named("b_semaphore0", 1);
    let c = Semaphore::new_named("c_semaphore0", 1);

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("t1", move || t1(a1, b1));

    let b2 = Arc::clone(&b);
    let c2 = Arc::clone(&c);
    let h2 = cir_trace::spawn("t2", move || t2(b2, c2));

    let a3 = Arc::clone(&a);
    let c3 = Arc::clone(&c);
    let h3 = cir_trace::spawn("t3", move || t3(a3, c3));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
