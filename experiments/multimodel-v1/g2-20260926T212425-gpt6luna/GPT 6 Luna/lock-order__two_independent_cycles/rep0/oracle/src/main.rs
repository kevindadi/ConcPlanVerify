mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn t1(a: Arc<Semaphore>, b: Arc<Semaphore>) {
    let _a = a.acquire();
    let _b = b.acquire();
}

fn t2(a: Arc<Semaphore>, b: Arc<Semaphore>) {
    let _a = a.acquire();
    let _b = b.acquire();
}

fn t3(c: Arc<Semaphore>, d: Arc<Semaphore>) {
    let _c = c.acquire();
    let _d = d.acquire();
}

fn t4(c: Arc<Semaphore>, d: Arc<Semaphore>) {
    let _c = c.acquire();
    let _d = d.acquire();
}

fn main() { cir_trace::init();
    let a = Semaphore::new_named("a_semaphore0", 1);
    let b = Semaphore::new_named("b_semaphore0", 1);
    let c = Semaphore::new_named("c_semaphore0", 1);
    let d = Semaphore::new_named("d_semaphore0", 1);

    let h1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t1", move || t1(a, b))
    };
    let h2 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t2", move || t2(a, b))
    };
    let h3 = {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        cir_trace::spawn("t3", move || t3(c, d))
    };
    let h4 = {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        cir_trace::spawn("t4", move || t4(c, d))
    };

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
