use std::sync::mpsc;
use std::thread;

fn main() {
    let (tx, rx) = mpsc::sync_channel(0);

    let sender = thread::spawn(move || {
        tx.send(1).unwrap();
    });

    let receiver = thread::spawn(move || {
        let value = rx.recv().unwrap();
        value
    });

    sender.join().unwrap();
    let done = receiver.join().unwrap();

    println!("DONE done={}", done);
}
