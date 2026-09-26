mod main {
    use std::thread;

    use concir_sync::Semaphore;

    pub fn main() {
        let s = Semaphore::new(1);

        let s1 = s.clone();
        let t1 = thread::spawn(move || w1(s1));

        let s2 = s.clone();
        let t2 = thread::spawn(move || w2(s2));

        t1.join().unwrap();
        t2.join().unwrap();

        println!("DONE permits={}", s.available_permits());
    }

    fn w1(s: concir_sync::Arc<Semaphore>) {
        let permit = s.acquire();
        let mut work = 0;
        work = 1;
        let _ = work;
        drop(permit);
    }

    fn w2(s: concir_sync::Arc<Semaphore>) {
        let permit = s.acquire();
        let mut work = 0;
        work = 1;
        let _ = work;
        drop(permit);
    }
}

fn main() {
    main::main();
}
