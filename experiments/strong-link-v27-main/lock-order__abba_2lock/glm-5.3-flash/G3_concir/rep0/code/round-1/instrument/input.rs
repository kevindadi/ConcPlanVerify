use std::sync::{Arc, Mutex};

fn t1(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) {
    // s1: mutex_lock main::a
    let _guard_a = a.lock().unwrap();
    // s2: mutex_lock main::b
    let _guard_b = b.lock().unwrap();
    // s3: assign_local critical_done = true
    let critical_done: bool = true;
    let _ = critical_done;
    // s4: mutex_unlock main::b (guard leaves scope below)
    // s5: mutex_unlock main::a
    drop(_guard_b);
    drop(_guard_a);
    // s6: return
}

fn t2(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) {
    // s1: mutex_lock main::a
    let _guard_a = a.lock().unwrap();
    // s2: mutex_lock main::b
    let _guard_b = b.lock().unwrap();
    // s3: assign_local critical_done = true
    let critical_done: bool = true;
    let _ = critical_done;
    // s4: mutex_unlock main::b
    // s5: mutex_unlock main::a
    drop(_guard_b);
    drop(_guard_a);
    // s6: return
}

fn main() {
    // resources: a, b
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    // s1: spawn main::t1 -> h1
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = std::thread::spawn(move || t1(&a1, &b1));

    // s2: spawn main::t2 -> h2
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = std::thread::spawn(move || t2(&a2, &b2));

    // s3: join h1
    h1.join().expect("t1 panicked");
    // s4: join h2
    h2.join().expect("t2 panicked");
    // s5: return

    // R9: terminal line
    println!("DONE t1=1 t2=1");
}
