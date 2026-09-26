mod main {
    use std::sync::Arc;

    use concir_sync::{thread, Semaphore};

    pub fn main() {
        let s = Arc::new(Semaphore::new(1));

        let s1 = Arc::clone(&s);
        let t1 = thread::spawn(move || w1(s1));

        let s2 = Arc::clone(&s);
        let t2 = thread::spawn(move || w2(s2));

        t1.join().unwrap();
        t2.join().unwrap();

        println!("DONE permits={}", s.available_permits());
    }

    fn w1(s: Arc<Semaphore>) {
        s.acquire().unwrap();
        let mut work = 0;
        work = 1;
        let _ = work;
        s.release();
    }

    fn w2(s: Arc<Semaphore>) {
        s.acquire().unwrap();
        let mut work = 0;
        work = 1;
        let _ = work;
        s.release();
    }
}

fn main() {
    main::main();
}
