use std::thread;

fn worker() {}

fn println(line: String) {
    std::println!("{line}");
}

fn main() {
    let mut again = false;
    loop {
        let h = thread::spawn(worker);
        h.join().unwrap();
        if again == true {
            println("DONE done=1".to_string());
            return;
        } else {
            again = true;
        }
    }
}
