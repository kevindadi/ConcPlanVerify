mod cir_trace;
#[allow(unused_imports)]
use concir_sync::Semaphore;

fn worker() {}

fn main() { cir_trace::init();
    let mut done = 0;
    while done < 1 {
        let handle = cir_trace::spawn("handle", worker);
        handle.join().unwrap();
        done += 1;
    }
    println!("DONE done={done}");
 cir_trace::finish();}
