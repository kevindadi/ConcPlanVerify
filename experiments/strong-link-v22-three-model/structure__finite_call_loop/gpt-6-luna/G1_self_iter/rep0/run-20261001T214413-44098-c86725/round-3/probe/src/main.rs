fn helper() {}

fn call_sequence() {
    std::thread::spawn(helper)
        .join()
        .expect("helper thread panicked");
}

fn main() {
    call_sequence();
    call_sequence();
    println!("DONE done=1");
}
