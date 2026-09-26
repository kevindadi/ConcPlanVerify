use concir_sync::Semaphore;
use std::sync::{Arc, Mutex};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>, sem: Arc<concir_sync::Semaphore>) {
    let _permit = sem.acquire();
    let _la = a.lock().unwrap();
    let _lb = b.lock().unwrap();
}

fn t2(b: Arc<Mutex<()>>, c: Arc<Mutex<()>>, sem: Arc<concir_sync::Semaphore>) {
    let _permit = sem.acquire();
    let _lb = b.lock().unwrap();
    let _lc = c.lock().unwrap();
}

fn t3(a: Arc<Mutex<()>>, c: Arc<Mutex<()>>, sem: Arc<concir_sync::Semaphore>) {
    let _permit = sem.acquire();
    let _la = a.lock().unwrap();
    let _lc = c.lock().unwrap();
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));
    let sem = Semaphore::new(1);

    let h1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let sem = Arc::clone(&sem);
        thread::spawn(move || t1(a, b, sem))
    };
    let h2 = {
        let b = Arc::clone(&b);
        let c = Arc::clone(&c);
        let sem = Arc::clone(&sem);
        thread::spawn(move || t2(b, c, sem))
    };
    let h3 = {
        let a = Arc::clone(&a);
        let c = Arc::clone(&c);
        let sem = Arc::clone(&sem);
        thread::spawn(move || t3(a, c, sem))
    };

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    println!("DONE done=1");
}
