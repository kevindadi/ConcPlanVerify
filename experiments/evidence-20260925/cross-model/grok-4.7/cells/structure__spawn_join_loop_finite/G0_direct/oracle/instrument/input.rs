#[allow(unused_imports)]
use concir_sync::Semaphore;

fn worker() {}

fn main() {
    let mut done = 0;
    while done < 1 {
        let handle = std::thread::spawn(worker);
        handle.join().unwrap();
        done += 1;
    }
    println!("DONE done={done}");
}
