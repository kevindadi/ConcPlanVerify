mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(m: &Arc<Mutex<i32>>) {
    // mutex_lock m
    let mut guard = m.lock().unwrap();
    // write_shared c = c + 1 (c lives inside m)
    *guard = *guard + 1;
    // mutex_unlock m (guard drops)
    drop(guard);
}

fn w2(m: &Arc<Mutex<i32>>) {
    // mutex_lock m
    let mut guard = m.lock().unwrap();
    // write_shared c = c + 1 (c lives inside m)
    *guard = *guard + 1;
    // mutex_unlock m (guard drops)
    drop(guard);
}

fn main() { cir_trace::init();
    // shared resource m (Mutex) guarding c, c starts at 0, range 0..=2
    let m: Arc<Mutex<i32>> = Arc::new(Mutex::new_named("m_mutex0#612", 0));

    // spawn h1: main::w1
    let m1 = Arc::clone(&m);
    let h1 = cir_trace::spawn("w1#690", move || w1(&m1));

    // spawn h2: main::w2
    let m2 = Arc::clone(&m);
    let h2 = cir_trace::spawn("w2#791", move || w2(&m2));

    // join h1
    h1.join().unwrap();
    // join h2
    h2.join().unwrap();

    // terminal line
    println!("DONE done=1");
 cir_trace::finish();}
