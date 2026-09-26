mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Main {
    m: Mutex<i32>,
}

fn w1(main: Arc<Main>) {
    {
        let mut guard = main.m.lock().unwrap();
        *guard = 1;
    }
}

fn w2(main: Arc<Main>) {
    {
        let mut guard = main.m.lock().unwrap();
        *guard = 1;
    }
}

fn main() { cir_trace::init();
    let main = Arc::new(Main { m: Mutex::new_named("main_mutex0", 0) });

    let m1 = Arc::clone(&main);
    let m2 = Arc::clone(&main);

    let h1 = cir_trace::spawn("w1", move || w1(m1));
    let h2 = cir_trace::spawn("w2", move || w2(m2));

    h1.join().unwrap();
    h2.join().unwrap();

    let done = *main.m.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
