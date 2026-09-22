use std::sync::{Arc, Mutex};
use std::thread;

static mut ACC: i32 = 0;

fn compute() {
    let x = 1;
    let _y = x + 1;
}

fn w1(m: Arc<Mutex<()>>) {
    {
        let _guard = m.lock().unwrap();
        compute();
        let tmp = unsafe { ACC };
        let tmp = tmp + 1;
        unsafe { ACC = tmp };
    }
}

fn w2(m: Arc<Mutex<()>>) {
    {
        let _guard = m.lock().unwrap();
        compute();
        let tmp = unsafe { ACC };
        let tmp = tmp + 1;
        unsafe { ACC = tmp };
    }
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let m1 = Arc::clone(&m);
    let m2 = Arc::clone(&m);
    let t1 = thread::spawn(move || w1(m1));
    let t2 = thread::spawn(move || w2(m2));
    t1.join().unwrap();
    t2.join().unwrap();
    let done = unsafe { ACC };
    println!("DONE done={}", done);
}
