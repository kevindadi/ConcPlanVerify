mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn activation(s: Arc<Semaphore>) {
    let permit = s.acquire();

    let mut work = 0usize;
    for i in 0..1000 {
        work = work.wrapping_add(i);
    }
    std::hint::black_box(work);

    drop(permit);
}

fn run_role(s: Arc<Semaphore>) {
    let a1_s = Arc::clone(&s);
    let a2_s = Arc::clone(&s);

    let a1 = cir_trace::spawn("activation", move || activation(a1_s));
    let a2 = cir_trace::spawn("activation", move || activation(a2_s));

    a1.join().unwrap();
    a2.join().unwrap();
}

fn w1(s: Arc<Semaphore>) {
    run_role(s);
}

fn w2(s: Arc<Semaphore>) {
    run_role(s);
}

fn w3(s: Arc<Semaphore>) {
    run_role(s);
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    let w1_s = Arc::clone(&s);
    let w2_s = Arc::clone(&s);
    let w3_s = Arc::clone(&s);

    let w1_handle = cir_trace::spawn("w1", move || w1(w1_s));
    let w2_handle = cir_trace::spawn("w2", move || w2(w2_s));
    let w3_handle = cir_trace::spawn("w3", move || w3(w3_s));

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    w3_handle.join().unwrap();

    let done = 1;
    println!("DONE done={done}");
 cir_trace::finish();}
