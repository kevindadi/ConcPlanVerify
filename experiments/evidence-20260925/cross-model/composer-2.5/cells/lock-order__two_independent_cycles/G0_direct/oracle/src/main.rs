mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn t1(a: Arc<Semaphore>, b: Arc<Semaphore>) {
    let _guard_a = a.acquire();
    let _guard_b = b.acquire();
}

fn t2(a: Arc<Semaphore>, b: Arc<Semaphore>) {
    let _guard_a = a.acquire();
    let _guard_b = b.acquire();
}

fn t3(c: Arc<Semaphore>, d: Arc<Semaphore>) {
    let _guard_c = c.acquire();
    let _guard_d = d.acquire();
}

fn t4(c: Arc<Semaphore>, d: Arc<Semaphore>) {
    let _guard_c = c.acquire();
    let _guard_d = d.acquire();
}

fn main() { cir_trace::init();
    let a = Semaphore::new_named("a_semaphore0", 1);
    let b = Semaphore::new_named("b_semaphore0", 1);
    let c = Semaphore::new_named("c_semaphore0", 1);
    let d = Semaphore::new_named("d_semaphore0", 1);

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("t1", move || t1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = cir_trace::spawn("t2", move || t2(a2, b2));

    let c3 = Arc::clone(&c);
    let d3 = Arc::clone(&d);
    let h3 = cir_trace::spawn("t3", move || t3(c3, d3));

    let c4 = Arc::clone(&c);
    let d4 = Arc::clone(&d);
    let h4 = cir_trace::spawn("t4", move || t4(c4, d4));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
