mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn w3(s: Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn supervisor(s: Arc<Semaphore>) {
    let h_w1 = cir_trace::spawn("h_w1", {
        let s = s.clone();
        move || w1(s)
    });
    let h_w2 = cir_trace::spawn("h_w2", {
        let s = s.clone();
        move || w2(s)
    });
    let h_w3 = cir_trace::spawn("h_w3", {
        let s = s.clone();
        move || w3(s)
    });
    h_w1.join().unwrap();
    h_w2.join().unwrap();
    h_w3.join().unwrap();
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 2);
    let h_sup = cir_trace::spawn("h_sup", {
        let s = s.clone();
        move || supervisor(s)
    });
    h_sup.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}
