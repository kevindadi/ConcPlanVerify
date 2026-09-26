use std::sync::{Arc, Mutex};
use std::thread;

use concir_sync::Semaphore;

fn main() {
    // c 的声明范围是 0..=2，初值为 0。任何可达状态都必须满足 c <= 1。
    let m = Arc::new(Mutex::new(0i32));
    let gate = Semaphore::new(1);

    let m1 = Arc::clone(&m);
    let gate1 = Arc::clone(&gate);
    let w1 = thread::spawn(move || {
        // 始终先拿信号量再拿 m，并在释放信号量之前先放开 m，避免相反的加锁顺序。
        let permit = gate1.acquire();
        {
            let mut c = m1.lock().unwrap();
            if *c < 1 {
                *c += 1;
            }
        }
        permit.release();
    });

    let m2 = Arc::clone(&m);
    let gate2 = Arc::clone(&gate);
    let w2 = thread::spawn(move || {
        let permit = gate2.acquire();
        {
            let mut c = m2.lock().unwrap();
            if *c < 1 {
                *c += 1;
            }
        }
        permit.release();
    });

    w1.join().unwrap();
    w2.join().unwrap();

    let c = *m.lock().unwrap();
    println!("DONE done={c}");
}
