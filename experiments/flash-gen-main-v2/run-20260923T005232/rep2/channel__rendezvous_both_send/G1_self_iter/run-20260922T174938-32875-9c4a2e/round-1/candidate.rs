use std::sync::mpsc;
use std::thread;

fn main() {
    let (tx, rx) = mpsc::sync_channel::<i32>(0);

    let s1 = thread::spawn(move || {
        tx.send(1).unwrap();
    });

    let r = thread::spawn(move || {
        let v = rx.recv().unwrap();
        v
    });

    s1.join().unwrap();
    let done = r.join().unwrap();
    println!("DONE done={}", done);
}
