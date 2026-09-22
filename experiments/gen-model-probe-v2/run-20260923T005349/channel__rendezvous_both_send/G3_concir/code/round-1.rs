use std::sync::mpsc::sync_channel;
use std::thread;

fn main() {
    let (tx, rx) = sync_channel::<i64>(0);

    let s1 = thread::spawn(move || {
        tx.send(1).unwrap();
    });

    let r = thread::spawn(move || {
        let got = rx.recv().unwrap();
        let _ = got;
    });

    s1.join().unwrap();
    r.join().unwrap();

    let done = 1;
    println!("DONE done={}", done);
}
