mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn activation(s: Arc<Mutex<()>>) {
    let _permit = s.lock().unwrap_or_else(|e| e.into_inner());

    let work = (0..1_000u64).fold(0u64, |sum, n| sum.wrapping_add(n));
    std::hint::black_box(work);
}

fn run_role(s: Arc<Mutex<()>>) {
    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let a1 = cir_trace::spawn("activation#357", move || activation(s1));
    let a2 = cir_trace::spawn("activation#409", move || activation(s2));

    a1.join().expect("activation panicked");
    a2.join().expect("activation panicked");
}

fn w1(s: Arc<Mutex<()>>) {
    run_role(s);
}

fn w2(s: Arc<Mutex<()>>) {
    run_role(s);
}

fn w3(s: Arc<Mutex<()>>) {
    run_role(s);
}

fn main() { cir_trace::init();
    let s = Arc::new(Mutex::new_named("s_mutex0#723", ()));

    let t1 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("w1#789", move || w1(s))
    };
    let t2 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("w2#880", move || w2(s))
    };
    let t3 = cir_trace::spawn("w3#929", move || w3(s));

    t1.join().expect("w1 panicked");
    t2.join().expect("w2 panicked");
    t3.join().expect("w3 panicked");

    println!("DONE done=1");
 cir_trace::finish();}
