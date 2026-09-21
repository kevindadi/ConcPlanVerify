use std::thread;

fn main() {
    for _ in 0..1 {
        let handle = thread::spawn(|| {
            // Worker performs no shared work.
        });
        handle.join().unwrap();
    }
    println!("DONE done=1");
}
