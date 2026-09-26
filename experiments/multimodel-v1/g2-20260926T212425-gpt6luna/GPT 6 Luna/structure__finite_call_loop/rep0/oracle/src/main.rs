mod cir_trace;
use concir_sync::Semaphore;

fn helper() {}

fn main() { cir_trace::init();
    let _ = std::mem::size_of::<Option<Semaphore>>();
    helper();
    helper();
    println!("DONE done=1");
 cir_trace::finish();}
