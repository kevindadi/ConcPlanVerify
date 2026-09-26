use concir_sync::Semaphore;
use std::thread;

fn main() {
    let s = Semaphore::new(2);

    let w1 = {
        let s = s.clone();
        move || {
            let permit = s.acquire();
            let mut work = 0;
            work = 1;
            permit.release();
            let _ = work;
        }
    };

    let w2 = {
        let s = s.clone();
        move || {
            let permit = s.acquire();
            let mut work = 0;
            work = 1;
            permit.release();
            let _ = work;
        }
    };

    let w3 = {
        let s = s.clone();
        move || {
            let permit = s.acquire();
            let mut work = 0;
            work = 1;
            permit.release();
            let _ = work;
        }
    };

    let h1 = thread::spawn(w1);
    let h2 = thread::spawn(w2);
    let h3 = thread::spawn(w3);

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
}
