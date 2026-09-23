mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    drop(permit);
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    drop(permit);
}

fn w3(s: Arc<Semaphore>) {
    let permit = s.acquire();
    drop(permit);
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);
    let mut done = 0;

    let h1 = {
        let s = s.clone();
        cir_trace::spawn("w1", move || w1(s))
    };
    let h2 = {
        let s = s.clone();
        cir_trace::spawn("w2", move || w2(s))
    };
    let h3 = {
        let s = s.clone();
        cir_trace::spawn("w3", move || w3(s))
    };

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    done = 1;
    println!("DONE done={}", done);
 cir_trace::finish();}
