mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn activation(s: &Arc<Semaphore>) {
    let mut work: i32 = 0;
    let permit = s.acquire();
    work = work + 1;
    permit.release();
}

fn w1(s: Arc<Semaphore>) {
    activation(&s);
    activation(&s);
}

fn w2(s: Arc<Semaphore>) {
    activation(&s);
    activation(&s);
}

fn w3(s: Arc<Semaphore>) {
    activation(&s);
    activation(&s);
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#450", 1);

    let h1 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("w1#514", move || w1(s))
    };

    let h2 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("w2#606", move || w2(s))
    };

    let h3 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("w3#698", move || w3(s))
    };

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
