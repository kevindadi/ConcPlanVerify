use concir_sync::Semaphore;
use std::sync::{Arc, Mutex};
use std::thread;

fn w1(m: Arc<Semaphore>, c: Arc<Mutex<u8>>) {
    let _permit = m.acquire();
    let mut guard = c.lock().unwrap();
    if *guard < 1 {
        *guard += 1;
    }
}

fn w2(m: Arc<Semaphore>, c: Arc<Mutex<u8>>) {
    let _permit = m.acquire();
    let mut guard = c.lock().unwrap();
    if *guard < 1 {
        *guard += 1;
    }
}

fn main() {
    let m = Semaphore::new(1);
    let c = Arc::new(Mutex::new(0u8));

    let supervisor = thread::spawn({
        let m = Arc::clone(&m);
        let c = Arc::clone(&c);
        move || {
            let h1 = thread::spawn({
                let m = Arc::clone(&m);
                let c = Arc::clone(&c);
                move || w1(m, c)
            });
            let h2 = thread::spawn({
                let m = Arc::clone(&m);
                let c = Arc::clone(&c);
                move || w2(m, c)
            });
            h1.join().unwrap();
            h2.join().unwrap();
        }
    });

    supervisor.join().unwrap();
    println!("DONE done=1");
}
