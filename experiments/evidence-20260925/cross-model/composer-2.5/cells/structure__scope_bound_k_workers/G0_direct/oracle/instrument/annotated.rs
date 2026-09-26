mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn activation(s: Arc<Semaphore>) {
    let _permit = s.acquire();
    // work while holding the single permit
}

fn w1(s: Arc<Semaphore>) {
    let s0 = Arc::clone(&s);
    let s1 = Arc::clone(&s);
    let a0 = cir_trace::spawn("activation", move || activation(s0));
    let a1 = cir_trace::spawn("activation", move || activation(s1));
    a0.join().unwrap();
    a1.join().unwrap();
}

fn w2(s: Arc<Semaphore>) {
    let s0 = Arc::clone(&s);
    let s1 = Arc::clone(&s);
    let a0 = cir_trace::spawn("activation", move || activation(s0));
    let a1 = cir_trace::spawn("activation", move || activation(s1));
    a0.join().unwrap();
    a1.join().unwrap();
}

fn w3(s: Arc<Semaphore>) {
    let s0 = Arc::clone(&s);
    let s1 = Arc::clone(&s);
    let a0 = cir_trace::spawn("activation", move || activation(s0));
    let a1 = cir_trace::spawn("activation", move || activation(s1));
    a0.join().unwrap();
    a1.join().unwrap();
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    let s_w1 = Arc::clone(&s);
    let s_w2 = Arc::clone(&s);
    let s_w3 = Arc::clone(&s);

    let h1 = cir_trace::spawn("w1", move || w1(s_w1));
    let h2 = cir_trace::spawn("w2", move || w2(s_w2));
    let h3 = cir_trace::spawn("w3", move || w3(s_w3));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
