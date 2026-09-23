use std::thread;

fn worker() {
    // no operations
}

fn print() {
    println!("DONE done=1");
}

fn main() {
    let h1 = thread::spawn(worker);
    h1.join().unwrap();
    let h2 = thread::spawn(worker);
    h2.join().unwrap();
    print();
}
