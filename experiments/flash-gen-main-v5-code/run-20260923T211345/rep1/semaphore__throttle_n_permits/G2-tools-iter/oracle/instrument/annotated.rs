mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 2);

    let mut handles = Vec::new();

    let s1 = Arc::clone(&s);
    handles.push(thread::spawn(move || {
        w1(s1);
    }));

    let s2 = Arc::clone(&s);
    handles.push(thread::spawn(move || {
        w2(s2);
    }));

    let s3 = Arc::clone(&s);
    handles.push(thread::spawn(move || {
        w3(s3);
    }));

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    // work
    drop(permit);
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    // work
    drop(permit);
}

fn w3(s: Arc<Semaphore>) {
    let permit = s.acquire();
    // work
    drop(permit);
}
