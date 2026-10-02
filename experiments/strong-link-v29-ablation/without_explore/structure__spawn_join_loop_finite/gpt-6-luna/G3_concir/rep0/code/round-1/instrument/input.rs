use std::thread;

fn worker() {}

fn main() {
    let handle = thread::spawn(move || worker());
    handle.join().unwrap();
    println!("DONE done=1");
}
