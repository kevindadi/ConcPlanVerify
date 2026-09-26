mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn t1(a: Arc<Semaphore>, b: Arc<Semaphore>) {
    let _pa = a.acquire();
    let _pb = b.acquire();
}

fn t2(a: Arc<Semaphore>, b: Arc<Semaphore>) {
    let _pa = a.acquire();
    let _pb = b.acquire();
}

fn t3(c: Arc<Semaphore>, d: Arc<Semaphore>) {
    let _pc = c.acquire();
    let _pd = d.acquire();
}

fn t4(c: Arc<Semaphore>, d: Arc<Semaphore>) {
    let _pc = c.acquire();
    let _pd = d.acquire();
}

fn main() { cir_trace::init();
    let a = Semaphore::new_named("a_semaphore0", 1);
    let b = Semaphore::new_named("b_semaphore0", 1);
    let c = Semaphore::new_named("c_semaphore0", 1);
    let d = Semaphore::new_named("d_semaphore0", 1);

    let h1 = cir_trace::spawn("h1", {
        let a = a.clone();
        let b = b.clone();
        move || t1(a, b)
    });
    let h2 = cir_trace::spawn("h2", {
        let a = a.clone();
        let b = b.clone();
        move || t2(a, b)
    });
    let h3 = cir_trace::spawn("h3", {
        let c = c.clone();
        let d = d.clone();
        move || t3(c, d)
    });
    let h4 = cir_trace::spawn("h4", {
        let c = c.clone();
        let d = d.clone();
        move || t4(c, d)
    });

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
