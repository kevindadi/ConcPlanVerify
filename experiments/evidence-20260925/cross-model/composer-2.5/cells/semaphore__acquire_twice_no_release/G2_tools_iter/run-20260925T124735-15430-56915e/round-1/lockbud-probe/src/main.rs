use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    {
        let _p = s.acquire();
        // critical section
    }
    {
        let _p = s.acquire();
        // critical section
    }
}

fn w2(s: Arc<Semaphore>) {
    {
        let _p = s.acquire();
        // critical section
    }
    {
        let _p = s.acquire();
        // critical section
    }
}

fn main() {
    let s = Semaphore::new(1);

    let s1 = Arc::clone(&s);
    let j1 = thread::spawn(move || w1(s1));

    let s2 = Arc::clone(&s);
    let j2 = thread::spawn(move || w2(s2));

    j1.join().unwrap();
    j2.join().unwrap();

    println!("DONE done=1");
}
