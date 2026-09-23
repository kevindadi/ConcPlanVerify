mod cir_trace;
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread;

fn main() {
    let (tx,rx)=mpsc::sync_channel::<i32>(1);
    cir_trace::ev(&cir_trace::tag_str(), "L1"); let s=thread::spawn(move||{cir_trace::set_tag("tL1");  cir_trace::ev(&cir_trace::tag_str(), "L2"); tx.send(1).unwrap(); cir_trace::ev(&cir_trace::tag_str(), "L3"); tx.send(2).unwrap(); });
    cir_trace::ev(&cir_trace::tag_str(), "L4"); let r=thread::spawn(move||{cir_trace::set_tag("tL4");  cir_trace::ev(&cir_trace::tag_str(), "L5"); rx.recv().unwrap(); cir_trace::ev(&cir_trace::tag_str(), "L6"); rx.recv().unwrap(); });
    cir_trace::ev(&cir_trace::tag_str(), "L7"); s.join().unwrap(); cir_trace::ev(&cir_trace::tag_str(), "L8"); r.join().unwrap();
    println!("DONE done=1");
cir_trace::finish(); }
