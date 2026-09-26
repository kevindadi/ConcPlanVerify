use std::thread;

fn worker() {
}

fn main() {
    let w1 = thread::spawn(|| worker());
    w1.join().unwrap();
    let w2 = thread::spawn(|| worker());
    w2.join().unwrap();
    println!("DONE done=1");
}
