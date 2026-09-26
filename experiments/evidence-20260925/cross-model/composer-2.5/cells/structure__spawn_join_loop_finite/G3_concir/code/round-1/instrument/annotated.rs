mod cir_trace;
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

fn main() { cir_trace::init();
    let h0 = cir_trace::spawn("h0", main::worker);
    h0.join().unwrap();
    let h1 = cir_trace::spawn("h1", main::worker);
    h1.join().unwrap();
    main::print_done();
 cir_trace::finish();}
