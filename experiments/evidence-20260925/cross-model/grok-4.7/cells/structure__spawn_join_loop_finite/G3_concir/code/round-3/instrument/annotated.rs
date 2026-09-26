mod cir_trace;
use std::thread;

fn worker() {}

fn println(line: String) {
    println!("{line}");
}

fn main() { cir_trace::init();
    let mut again = false;
    loop {
        let h = cir_trace::spawn("h", worker);
        h.join().unwrap();
        if again == true {
            println("DONE done=1".to_string());
            return;
        } else {
            again = true;
        }
    }
 cir_trace::finish();}
