mod cir_trace;
fn worker() {}

fn main() { cir_trace::init();
    let worker1 = std::thread::Builder::new()
        .name("worker1".to_string())
        .spawn(worker)
        .unwrap();
    worker1.join().unwrap();

    let worker2 = std::thread::Builder::new()
        .name("worker2".to_string())
        .spawn(worker)
        .unwrap();
    worker2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
