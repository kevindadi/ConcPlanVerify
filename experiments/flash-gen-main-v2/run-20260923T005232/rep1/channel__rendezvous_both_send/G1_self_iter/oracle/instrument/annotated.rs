mod cir_trace;
use std::sync::mpsc;
use std::thread;

fn main() { cir_trace::init();
    let (tx, rx) = mpsc::sync_channel::<i32>(0);

    let s1 = cir_trace::spawn("s1", move || {
        tx.send(1).unwrap();
    });

    let r = cir_trace::spawn("r", move || {
        let v = rx.recv().unwrap();
        v
    });

    s1.join().unwrap();
    let done = r.join().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
