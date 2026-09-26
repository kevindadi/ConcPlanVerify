mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

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
    let h1 = cir_trace::spawn("h1", {
        let s = Arc::clone(&s);
        move || w1(s)
    });
    let h2 = cir_trace::spawn("h2", {
        let s = Arc::clone(&s);
        move || w2(s)
    });
    let h3 = cir_trace::spawn("h3", {
        let s = Arc::clone(&s);
        move || w3(s)
    });
    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
}

fn println() {
    std::println!("DONE done=1");
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 2);
    let h_sup = cir_trace::spawn("h_sup", {
        let s = Arc::clone(&s);
        move || supervisor(s)
    });
    h_sup.join().unwrap();
    println();
 cir_trace::finish();}
