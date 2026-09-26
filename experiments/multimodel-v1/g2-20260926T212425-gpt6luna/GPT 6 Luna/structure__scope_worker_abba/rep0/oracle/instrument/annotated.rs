mod cir_trace;
use concir_sync::Semaphore;
use std::sync::{Arc, Barrier};
use std::thread;

fn w1(a: Arc<Semaphore>, b: Arc<Semaphore>, start: Arc<Barrier>) {
    start.wait();
    let _a = a.acquire();
    let _b = b.acquire();
}

fn w2(a: Arc<Semaphore>, b: Arc<Semaphore>, start: Arc<Barrier>) {
    start.wait();
    let _a = a.acquire();
    let _b = b.acquire();
}

fn main() { cir_trace::init();
    let a = Semaphore::new_named("a_semaphore0", 1);
    let b = Semaphore::new_named("b_semaphore0", 1);
    let start = Arc::new(Barrier::new(2));

    let worker1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let start = Arc::clone(&start);
        cir_trace::spawn("w1", move || w1(a, b, start))
    };

    let worker2 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let start = Arc::clone(&start);
        cir_trace::spawn("w2", move || w2(a, b, start))
    };

    worker1.join().unwrap();
    worker2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
