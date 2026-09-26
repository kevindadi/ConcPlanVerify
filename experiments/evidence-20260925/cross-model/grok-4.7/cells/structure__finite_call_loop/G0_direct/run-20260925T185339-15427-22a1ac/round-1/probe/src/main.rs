use std::thread;

#[allow(unused_imports)]
use concir_sync::Semaphore;

fn helper() {}

fn main() {
    let mut remaining = 2;
    while remaining > 0 {
        thread::spawn(helper).join().unwrap();
        remaining -= 1;
    }
    println!("DONE done=1");
}
