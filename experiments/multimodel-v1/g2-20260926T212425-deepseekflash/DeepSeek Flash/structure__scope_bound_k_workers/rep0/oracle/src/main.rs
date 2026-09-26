mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    let mut handles = Vec::new();

    for _ in 0..2 {
        let s1 = Arc::clone(&s);
        handles.push(thread::spawn(move || {
            let permit = s1.acquire();
            drop(permit);
        }));
    }

    for _ in 0..2 {
        let s2 = Arc::clone(&s);
        handles.push(thread::spawn(move || {
            let permit = s2.acquire();
            drop(permit);
        }));
    }

    for _ in 0..2 {
        let s3 = Arc::clone(&s);
        handles.push(thread::spawn(move || {
            let permit = s3.acquire();
            drop(permit);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
