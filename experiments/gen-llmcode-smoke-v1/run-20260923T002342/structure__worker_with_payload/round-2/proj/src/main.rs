mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

static mut ACC: i32 = 0;

fn w2() -> i32 {
    let local = 1;
    local
}

fn w1(m: &Mutex<()>) {
    {
        let _guard = m.lock().unwrap();
        let _ = w2();
        let tmp = unsafe { ACC };
        let tmp2 = tmp + 1;
        unsafe { ACC = tmp2 };
    }
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let m1 = Arc::clone(&m);
    let m2 = Arc::clone(&m);
    let h1 = cir_trace::spawn("h1", move || {
        w1(&m1);
    });
    let h2 = cir_trace::spawn("h2", move || {
        w1(&m2);
    });
    h1.join().unwrap();
    h2.join().unwrap();
    let done = unsafe { ACC };
    println!("DONE done={}", done);
 cir_trace::finish();}
