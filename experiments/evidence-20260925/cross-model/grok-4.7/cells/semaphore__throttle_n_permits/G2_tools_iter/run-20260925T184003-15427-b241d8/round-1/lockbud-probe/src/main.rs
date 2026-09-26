use concir_sync::Semaphore;
use std::thread;

fn main() {
    let s = Semaphore::new(2);

    let s1 = s.clone();
    let w1 = thread::spawn(move || {
        let permit = s1.acquire();
        let mut work = 0u32;
        for i in 0..8u32 {
            work = work.wrapping_add(i);
        }
        let _ = work;
        permit.release();
    });

    let s2 = s.clone();
    let w2 = thread::spawn(move || {
        let permit = s2.acquire();
        let mut work = 0u32;
        for i in 0..8u32 {
            work = work.wrapping_add(i);
        }
        let _ = work;
        permit.release();
    });

    let s3 = s.clone();
    let w3 = thread::spawn(move || {
        let permit = s3.acquire();
        let mut work = 0u32;
        for i in 0..8u32 {
            work = work.wrapping_add(i);
        }
        let _ = work;
        permit.release();
    });

    w1.join().unwrap();
    w2.join().unwrap();
    w3.join().unwrap();

    println!("DONE done=1");
}
