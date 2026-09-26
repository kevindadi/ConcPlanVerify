use concir_sync::Semaphore;
use std::sync::mpsc;
use std::thread;

fn main() {
    let ch = mpsc::sync_channel::<i32>(0);
    let start = Semaphore::new(2);

    let tx = ch.0;
    let start_s1 = start.clone();
    let s1 = thread::spawn(move || {
        let _permit = start_s1.acquire();
        let _ = tx.send(1);
    });

    let rx = ch.1;
    let start_r = start.clone();
    let r = thread::spawn(move || {
        let _permit = start_r.acquire();
        let _ = rx.recv().unwrap();
    });

    s1.join().unwrap();
    r.join().unwrap();
    println!("DONE done=1");
}
