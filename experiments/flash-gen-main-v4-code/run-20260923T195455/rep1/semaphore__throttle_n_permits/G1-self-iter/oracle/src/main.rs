mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 2);

    let mut handles = Vec::new();

    let s1 = Arc::clone(&s);
    handles.push(thread::spawn(move || {
        let _permit = s1.acquire();
        // work
    }));

    let s2 = Arc::clone(&s);
    handles.push(thread::spawn(move || {
        let _permit = s2.acquire();
        // work
    }));

    let s3 = Arc::clone(&s);
    handles.push(thread::spawn(move || {
        let _permit = s3.acquire();
        // work
    }));

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
