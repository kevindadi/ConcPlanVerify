use std::thread;

fn main() {
    for _ in 0..1 {
        let handle = thread::spawn(|| {
            // R2: worker performs no shared work
        });
        // R3: wait exactly once before next iteration
        handle.join().unwrap();
    }
    println!("DONE done=1");
}
