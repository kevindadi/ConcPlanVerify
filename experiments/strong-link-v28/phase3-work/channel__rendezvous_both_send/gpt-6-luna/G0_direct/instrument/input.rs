use std::sync::mpsc::sync_channel;
use std::thread;

fn main() {
    let ch = sync_channel::<()>(0);
    let (tx, rx) = ch;

    let s1 = thread::spawn(move || {
        tx.send(()).unwrap();
    });

    let r = thread::spawn(move || {
        rx.recv().unwrap();
    });

    s1.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
}
