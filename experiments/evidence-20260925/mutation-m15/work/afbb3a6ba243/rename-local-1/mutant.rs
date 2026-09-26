use std::thread;

fn worker() {
}

fn main() {
    let h1 = thread::spawn(worker);
    h1.join().unwrap();
    let h2_kept = thread::spawn(worker);
    h2_kept.join().unwrap();
    println!("DONE done=1");
}
