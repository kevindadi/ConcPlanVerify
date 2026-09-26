use std::sync::Arc;

use cir_trace::{join, spawn};
use concir_sync::Semaphore;

fn w1(s: Arc<Semaphore>) {
    s.acquire();
    let mut done = 0;
    done = 1;
    let _ = done;
    s.release();
}

fn w2(s: Arc<Semaphore>) {
    s.acquire();
    let mut done = 0;
    done = 1;
    let _ = done;
    s.release();
}

fn w3(s: Arc<Semaphore>) {
    s.acquire();
    let mut done = 0;
    done = 1;
    let _ = done;
    s.release();
}

fn main() {
    let s = Semaphore::new(2);

    let s1 = Arc::clone(&s);
    let h1 = spawn("w1", move || w1(s1));

    let s2 = Arc::clone(&s);
    let h2 = spawn("w2", move || w2(s2));

    let s3 = Arc::clone(&s);
    let h3 = spawn("w3", move || w3(s3));

    join(h1).unwrap();
    join(h2).unwrap();
    join(h3).unwrap();

    println!("DONE done=1");
}
