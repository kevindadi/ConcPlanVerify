use std::sync::{Arc, Mutex};

fn t1(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    let work: i32 = 1;
    let _ = work;
    drop(_gb);
    drop(_ga);
}

fn t2(b: &Arc<Mutex<()>>, c: &Arc<Mutex<()>>) {
    let _gb = b.lock().unwrap();
    let _gc = c.lock().unwrap();
    let work: i32 = 2;
    let _ = work;
    drop(_gc);
    drop(_gb);
}

fn t3(a: &Arc<Mutex<()>>, c: &Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gc = c.lock().unwrap();
    let work: i32 = 3;
    let _ = work;
    drop(_gc);
    drop(_ga);
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));

    let h1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        std::thread::spawn(move || t1(&a, &b))
    };
    let h2 = {
        let b = Arc::clone(&b);
        let c = Arc::clone(&c);
        std::thread::spawn(move || t2(&b, &c))
    };
    let h3 = {
        let a = Arc::clone(&a);
        let c = Arc::clone(&c);
        std::thread::spawn(move || t3(&a, &c))
    };

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    let done: i32 = 1;
    println!("DONE done={}", done);
}
