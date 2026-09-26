mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

use concir_sync::Semaphore;

fn with_slot(gate: &Arc<Semaphore>) {
    if let Some(permit) = gate.try_acquire() {
        drop(permit);
    }
}

fn bump(m: &Mutex<i32>) {
    let mut c = m.lock().unwrap();
    if *c < 1 {
        *c += 1;
    }
}

fn main() { cir_trace::init();
    // c 的声明范围是 0..=2，初值为 0。任何可达状态都必须满足 c <= 1。
    let m = Arc::new(Mutex::new_named("m_mutex0", 0i32));
    let gate = Semaphore::new_named("gate_semaphore0", 1);

    let m1 = Arc::clone(&m);
    let gate1 = Arc::clone(&gate);
    let w1 = cir_trace::spawn("with_slot", move || {
        with_slot(&gate1);
        bump(&m1);
    });

    let m2 = Arc::clone(&m);
    let gate2 = Arc::clone(&gate);
    let w2 = cir_trace::spawn("with_slot", move || {
        with_slot(&gate2);
        bump(&m2);
    });

    w1.join().unwrap();
    w2.join().unwrap();

    let c = *m.lock().unwrap();
    println!("DONE done={c}");
 cir_trace::finish();}
