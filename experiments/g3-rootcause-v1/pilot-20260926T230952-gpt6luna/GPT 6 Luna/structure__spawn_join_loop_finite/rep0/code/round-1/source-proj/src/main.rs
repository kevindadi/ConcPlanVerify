fn worker() {}

fn main() {
    let worker1 = std::thread::spawn(worker);
    worker1.join().unwrap();

    let worker2 = std::thread::spawn(worker);
    worker2.join().unwrap();

    println!("DONE done=1");
}
