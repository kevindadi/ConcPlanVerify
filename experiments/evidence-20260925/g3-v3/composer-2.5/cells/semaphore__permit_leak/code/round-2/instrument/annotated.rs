mod cir_trace;
mod main {
    use std::sync::Arc;

    use concir_sync::{thread, Semaphore};

    pub fn main() {
        let s = Arc::new(Semaphore::new_named("s_semaphore0", 1));

        let s1 = Arc::clone(&s);
        let t1 = cir_trace::spawn("w1", move || w1(s1));

        let s2 = Arc::clone(&s);
        let t2 = cir_trace::spawn("w2", move || w2(s2));

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

fn main() { cir_trace::init();
    main::main();
 cir_trace::finish();}
