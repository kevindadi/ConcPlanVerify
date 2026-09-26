use std::thread;

mod io {
    pub fn println() {
        std::println!("DONE done=1");
    }
}

mod main {
    pub fn worker() {}

    pub fn print_done() {
        crate::io::println();
    }
}

fn main() {
    let h0 = thread::spawn(main::worker);
    h0.join().unwrap();
    let h1 = thread::spawn(main::worker);
    h1.join().unwrap();
    main::print_done();
}
