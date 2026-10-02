use std::thread;

fn worker() {}

fn main() {
    let h1 = thread::spawn(|| {
        worker();
    });
    h1.join().expect("worker h1 panicked");

    let h2 = thread::spawn(|| {
        worker();
    });
    h2.join().expect("worker h2 panicked");

    println!("DONE done=1");
}
