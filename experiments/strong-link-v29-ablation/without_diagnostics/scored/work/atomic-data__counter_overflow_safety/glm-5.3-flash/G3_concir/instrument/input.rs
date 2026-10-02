use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};

fn w1(m: &Arc<Mutex<()>>, c: &Arc<AtomicI32>) {
    // s1: mutex_lock m
    let guard = m.lock().unwrap();
    // s2: atomic_load c -> v
    let v = c.load(Ordering::SeqCst);
    // s3: branch v < 1
    if v < 1 {
        // s4: atomic_cas c expected v desired v + 1
        let _ = c.compare_exchange(v, v + 1, Ordering::SeqCst, Ordering::SeqCst);
    }
    // s5: mutex_unlock m (guard drops here)
    drop(guard);
    // s6: return
}

fn w2(m: &Arc<Mutex<()>>, c: &Arc<AtomicI32>) {
    // s1: mutex_lock m
    let guard = m.lock().unwrap();
    // s2: atomic_load c -> v
    let v = c.load(Ordering::SeqCst);
    // s3: branch v < 1
    if v < 1 {
        // s4: atomic_cas c expected v desired v + 1
        let _ = c.compare_exchange(v, v + 1, Ordering::SeqCst, Ordering::SeqCst);
    }
    // s5: mutex_unlock m (guard drops here)
    drop(guard);
    // s6: return
}

fn main() {
    // Resources: m (Mutex), c (shared var, atomic int, init 0)
    let m: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
    let c: Arc<AtomicI32> = Arc::new(AtomicI32::new(0));

    // scope: start w1 and w2
    let m1 = Arc::clone(&m);
    let c1 = Arc::clone(&c);
    let h1 = std::thread::spawn(move || w1(&m1, &c1));

    let m2 = Arc::clone(&m);
    let c2 = Arc::clone(&c);
    let h2 = std::thread::spawn(move || w2(&m2, &c2));

    // wait for both workers to finish
    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    // terminal line
    println!("DONE done=1");
}
