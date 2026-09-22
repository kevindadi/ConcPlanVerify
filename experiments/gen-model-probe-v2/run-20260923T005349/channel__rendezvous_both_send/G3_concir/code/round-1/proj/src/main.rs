mod cir_trace;
use std::sync::mpsc::sync_channel;
use std::thread;

fn main() { cir_trace::init();
    let (tx, rx) = sync_channel::<i64>(0);

    let s1 = cir_trace::spawn("s1", move || {
        tx.send(1).unwrap();
    });

    let r = cir_trace::spawn("r", move || {
        let got = rx.recv().unwrap();
        let _ = got;
    });

    s1.join().unwrap();
    r.join().unwrap();

    let done = 1;
    println!("DONE done={}", done);
 cir_trace::finish();}
