mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;

fn w1(s: Arc<Semaphore>) {
    let _permit = s.acquire();
}

fn w2(s: Arc<Semaphore>) {
    let _permit = s.acquire();
}

fn w3(s: Arc<Semaphore>) {
    let _permit = s.acquire();
}

fn supervisor(s: Arc<Semaphore>) {
    let h1 = cir_trace::spawn("h1", {
        let s = s.clone();
        move || w1(s)
    });
    let h2 = cir_trace::spawn("h2", {
        let s = s.clone();
        move || w2(s)
    });
    let h3 = cir_trace::spawn("h3", {
        let s = s.clone();
        move || w3(s)
    });
    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
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
