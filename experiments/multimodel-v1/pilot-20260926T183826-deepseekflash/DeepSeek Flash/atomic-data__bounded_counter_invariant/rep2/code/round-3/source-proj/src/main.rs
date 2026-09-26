use std::sync::{Arc, Mutex};
use std::thread;

struct Main {
    m: Mutex<i32>,
}

fn w1(main: Arc<Main>) {
    let mut c = main.m.lock().unwrap();
    *c = *c + 1;
}

fn w2(main: Arc<Main>) {
    let mut c = main.m.lock().unwrap();
    *c = *c + 1;
}

fn main() {
    let main = Arc::new(Main { m: Mutex::new(0) });

    let m1 = Arc::clone(&main);
    let m2 = Arc::clone(&main);

    let h1 = thread::spawn(move || w1(m1));
    let h2 = thread::spawn(move || w2(m2));

    h1.join().unwrap();
    h2.join().unwrap();

    let done = *main.m.lock().unwrap();
    println!("DONE done={}", done);
}
