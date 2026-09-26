mod cir_trace;
mod main_mod {
    pub fn worker() {}

    pub fn main() {
        let h1 = cir_trace::spawn("h1", worker);
        h1.join().unwrap();

        let h2 = cir_trace::spawn("h2", worker);
        h2.join().unwrap();

        println!("DONE done=1");
    }
}

fn main() { cir_trace::init();
    main_mod::main();
 cir_trace::finish();}
