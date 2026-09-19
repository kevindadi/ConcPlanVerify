use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread;

fn main() {
    let (tx1,rx1)=mpsc::sync_channel::<i32>(0);
    let (tx2,_rx2)=mpsc::sync_channel::<i32>(0);
    let s=thread::spawn(move||{ tx1.send(1).unwrap(); let _=tx2; });
    let r=thread::spawn(move||{ let _=rx1; });
    s.join().unwrap(); r.join().unwrap();
    println!("DONE done=1");
}
