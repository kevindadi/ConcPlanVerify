use std::thread;

fn worker() {
}

fn main() {
    let h1_kept = thread::spawn(worker);
    h1_kept.join().unwrap();

    let h2 = thread::spawn(worker);
    h2.join().unwrap();

    println!("DONE done=1");
}
