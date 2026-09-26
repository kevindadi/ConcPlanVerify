mod main_mod {
    pub fn worker() {}

    pub fn main() {
        let h1 = std::thread::spawn(worker);
        h1.join().unwrap();

        let h2 = std::thread::spawn(worker);
        h2.join().unwrap();

        println!("DONE done=1");
    }
}

fn main() {
    main_mod::main();
}
