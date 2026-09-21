mod cir_trace;
use std::sync::mpsc;
use std::thread;

fn main() { cir_trace::init();
    let (tx, rx) = mpsc::sync_channel(0);

    let sender = cir_trace::spawn("sender", move || {
        tx.send(1).unwrap();
    });

    let receiver = cir_trace::spawn("receiver", move || {
        let value = rx.recv().unwrap();
        value
    });

    sender.join().unwrap();
    let done = receiver.join().unwrap();

    println!("DONE done={}", done);
 cir_trace::finish();}
