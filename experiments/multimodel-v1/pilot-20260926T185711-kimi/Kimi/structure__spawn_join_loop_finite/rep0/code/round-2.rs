use std::thread;

fn worker() {
    // no operations
}

fn main() {
    let h1 = thread::Builder::new()
        .name("h1".to_string())
        .spawn(worker)
        .expect("spawn worker h1");
    h1.join().unwrap();
    let h2 = thread::Builder::new()
        .name("h2".to_string())
        .spawn(worker)
        .expect("spawn worker h2");
    h2.join().unwrap();
    let done = 1;
    println!("DONE done={}", done);
}
