use std::sync::mpsc::sync_channel;
use std::thread;

fn main() {
    let (tx, rx) = sync_channel::<i64>(0);

    let s1 = thread::spawn(move || {
        tx.send(1).expect("send failed");
    });

    let r = thread::spawn(move || {
        let got: i64 = rx.recv().expect("recv failed");
        let _ = got;
    });

    s1.join().expect("s1 panicked");
    r.join().expect("r panicked");

    let done: i64 = 1;
    println!("DONE done={}", done);
}
