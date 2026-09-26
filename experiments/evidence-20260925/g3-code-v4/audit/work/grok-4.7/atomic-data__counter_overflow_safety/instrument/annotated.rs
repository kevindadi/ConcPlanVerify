mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};

use std::thread;

const C_MIN: i32 = 0;
const C_MAX: i32 = 2;

struct Shared {
    c: i32,
}

fn main() { cir_trace::init();
    let m = Mutex::new_named("m_mutex0", Shared { c: C_MIN });
    debug_assert!((C_MIN..=C_MAX).contains(&m.lock().unwrap().c));

    thread::scope(|scope| {
        scope.spawn(|| w1(&m));
        scope.spawn(|| w2(&m));
    });

    let done = m.lock().unwrap().c;
    println!("DONE done={done}");
 cir_trace::finish();}

fn w1(m: &Mutex<Shared>) {
    let mut guard = m.lock().unwrap();
    let cur = guard.c;
    if cur + 1 <= 1 {
        guard.c = cur + 1;
    }
}

fn w2(m: &Mutex<Shared>) {
    let mut guard = m.lock().unwrap();
    let cur = guard.c;
    if cur + 1 <= 1 {
        guard.c = cur + 1;
    }
}
