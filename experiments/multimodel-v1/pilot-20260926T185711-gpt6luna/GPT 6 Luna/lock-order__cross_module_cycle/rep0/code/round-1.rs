use std::sync::Mutex;
use std::thread;

pub static a: Mutex<()> = Mutex::new(());

mod other {
    use std::sync::Mutex;

    pub static b: Mutex<()> = Mutex::new(());

    pub fn t2() {
        let a_guard = crate::a.lock().unwrap();
        let b_guard = b.lock().unwrap();
        drop(b_guard);
        drop(a_guard);
    }
}

fn t1() {
    let a_guard = a.lock().unwrap();
    let b_guard = other::b.lock().unwrap();
    drop(b_guard);
    drop(a_guard);
}

fn main() {
    let t1_handle = thread::spawn(t1);
    let t2_handle = thread::spawn(other::t2);

    t1_handle.join().unwrap();
    t2_handle.join().unwrap();

    println!("DONE done=1");
}
