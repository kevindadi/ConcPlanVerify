use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let c = Arc::new(Mutex::new(0i32));
    let m = Arc::new(Mutex::new(()));

    let c_w1 = Arc::clone(&c);
    let m_w1 = Arc::clone(&m);

    let sup = thread::spawn(move || {
        let c_w2a = Arc::clone(&c_w1);
        let m_w2a = Arc::clone(&m_w1);
        let w1 = thread::spawn(move || {
            let _guard = m_w2a.lock().unwrap();
            let tmp = *c_w2a.lock().unwrap();
            let tmp2 = tmp + 1;
            *c_w2a.lock().unwrap() = tmp2;
        });

        let c_w2b = Arc::clone(&c_w1);
        let m_w2b = Arc::clone(&m_w1);
        let w2 = thread::spawn(move || {
            let _guard = m_w2b.lock().unwrap();
            let tmp = *c_w2b.lock().unwrap();
            let tmp2 = tmp + 1;
            *c_w2b.lock().unwrap() = tmp2;
        });

        w1.join().unwrap();
        w2.join().unwrap();
    });

    sup.join().unwrap();

    let done = *c.lock().unwrap();
    println!("DONE done={}", done);
}
