use std::thread;

fn worker() {}

fn main() {
    let worker1 = thread::spawn(worker);
    worker1.join().unwrap();

    let worker2 = thread::spawn(worker);
    worker2.join().unwrap();

    println!("DONE done=1");
}
