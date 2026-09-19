mod cir_trace {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNT: AtomicUsize = AtomicUsize::new(0);

    pub fn ev(_tag: &str, _sid: &str) {
        COUNT.fetch_add(1, Ordering::SeqCst);
    }

    pub fn finish() {
        let _ = COUNT.load(Ordering::SeqCst);
    }
}

use std::sync::{Arc, Mutex};

struct Shared {
    m: Mutex<i64>,
    acc: i64,
}

fn compute() {}

fn w1(shared: &Arc<Shared>) {
    {
        let mut guard = shared.m.lock().unwrap();
        cir_trace::ev("t1_0", "s1");
        compute();
        *guard = *guard + 1;
        cir_trace::ev("t1_0", "s4");
    }
    cir_trace::ev("t1_0", "s5");
}

fn w2(shared: &Arc<Shared>) {
    {
        let mut guard = shared.m.lock().unwrap();
        cir_trace::ev("t2_0", "s1");
        compute();
        *guard = *guard + 1;
        cir_trace::ev("t2_0", "s4");
    }
    cir_trace::ev("t2_0", "s5");
}

fn main() {
    let shared = Arc::new(Shared {
        m: Mutex::new(0),
        acc: 0,
    });

    cir_trace::ev("t0", "s1");

    let s1 = Arc::clone(&shared);
    let h1 = std::thread::spawn(move || {
        w1(&s1);
    });

    let s2 = Arc::clone(&shared);
    let h2 = std::thread::spawn(move || {
        w2(&s2);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    cir_trace::ev("t0", "s2");
    cir_trace::finish();
}
