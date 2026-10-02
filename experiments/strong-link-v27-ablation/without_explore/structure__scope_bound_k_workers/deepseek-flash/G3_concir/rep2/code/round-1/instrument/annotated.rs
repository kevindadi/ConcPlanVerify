mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn w3(s: Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#347", 1);

    let mut handles = Vec::new();

    let s1 = Arc::clone(&s);
    handles.push(thread::spawn(move || w1(s1)));
    let s2 = Arc::clone(&s);
    handles.push(thread::spawn(move || w1(s2)));
    let s3 = Arc::clone(&s);
    handles.push(thread::spawn(move || w2(s3)));
    let s4 = Arc::clone(&s);
    handles.push(thread::spawn(move || w2(s4)));
    let s5 = Arc::clone(&s);
    handles.push(thread::spawn(move || w3(s5)));
    let s6 = Arc::clone(&s);
    handles.push(thread::spawn(move || w3(s6)));

    for handle in handles {
        handle.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
