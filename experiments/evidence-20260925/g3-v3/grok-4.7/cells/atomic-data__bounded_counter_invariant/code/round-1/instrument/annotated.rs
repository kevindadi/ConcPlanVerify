mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};

use std::thread;

/// Shared counter `c`, declared to range from 0 to 2, protected by `m`.
struct Shared {
    c: i32,
}

fn main() { cir_trace::init();
    let m = Mutex::new_named("m_mutex0", Shared { c: 0 });
    thread::scope(|scope| {
        scope.spawn(|| w1(&m));
        scope.spawn(|| w2(&m));
    });
    println!("DONE done=1");
 cir_trace::finish();}

fn w1(m: &Mutex<Shared>) {
    let mut tmp: i32 = 0;
    {
        let mut guard = m.lock().unwrap();
        tmp = guard.c;
        tmp = tmp + 1;
        guard.c = tmp;
        assert!(guard.c >= 0 && guard.c <= 2);
    }
}

fn w2(m: &Mutex<Shared>) {
    let mut tmp: i32 = 0;
    {
        let mut guard = m.lock().unwrap();
        tmp = guard.c;
        tmp = tmp + 1;
        guard.c = tmp;
        assert!(guard.c >= 0 && guard.c <= 2);
    }
}
