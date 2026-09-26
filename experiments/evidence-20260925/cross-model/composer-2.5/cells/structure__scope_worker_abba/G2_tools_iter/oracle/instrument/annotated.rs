mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(a: Arc<Semaphore>, b: Arc<Semaphore>) {
    let _perm_a = a.acquire();
    let _perm_b = b.acquire();
}

fn w2(a: Arc<Semaphore>, b: Arc<Semaphore>) {
    let _perm_a = a.acquire();
    let _perm_b = b.acquire();
}

fn main() { cir_trace::init();
    let a = Semaphore::new_named("a_semaphore0", 1);
    let b = Semaphore::new_named("b_semaphore0", 1);

    let a_w2 = Arc::clone(&a);
    let b_w2 = Arc::clone(&b);

    let h1 = cir_trace::spawn("w1", move || w1(a, b));
    let h2 = cir_trace::spawn("w2", move || w2(a_w2, b_w2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
