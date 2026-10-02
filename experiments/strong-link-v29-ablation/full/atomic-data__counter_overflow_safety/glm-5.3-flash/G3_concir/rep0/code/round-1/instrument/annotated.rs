mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(m: &Arc<Mutex<i32>>) {
    // s1: mutex_lock m
    let mut guard = m.lock().unwrap();
    // s2: branch c < 1
    if *guard < 1 {
        // s3: write_shared c = c + 1
        *guard = *guard + 1;
    }
    // s4: mutex_unlock m (guard drops)
    drop(guard);
    // s5: return
}

fn w2(m: &Arc<Mutex<i32>>) {
    // s1: mutex_lock m
    let mut guard = m.lock().unwrap();
    // s2: branch c < 1
    if *guard < 1 {
        // s3: write_shared c = c + 1
        *guard = *guard + 1;
    }
    // s4: mutex_unlock m (guard drops)
    drop(guard);
    // s5: return
}

fn main() { crate::cir_trace::init();
    // shared resource c (init 0) protected by mutex m
    let m: Arc<Mutex<i32>> = Arc::new(Mutex::new_named("m_mutex0#733", 0));

    let m1 = Arc::clone(&m);
    let h1 = crate::cir_trace::spawn("w1#785", move || w1(&m1));

    let m2 = Arc::clone(&m);
    let h2 = crate::cir_trace::spawn("w2#860", move || w2(&m2));

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    // terminal line
    let final_c = m.lock().unwrap();
    println!("DONE done={}", *final_c);
 crate::cir_trace::finish();}
