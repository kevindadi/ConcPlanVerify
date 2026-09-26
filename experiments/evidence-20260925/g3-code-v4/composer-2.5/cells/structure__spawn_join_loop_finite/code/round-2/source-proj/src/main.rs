mod main {
    pub fn worker() {}

    pub fn main() {
        let h1 = std::thread::spawn(main::worker);
        h1.join().unwrap();

        let h2 = std::thread::spawn(main::worker);
        h2.join().unwrap();

        println!("DONE done=1");
    }
}

fn main() {
    main::main();
}
