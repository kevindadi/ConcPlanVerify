mod cir_trace;
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread;

fn main() {
    let (tx1,rx1)=mpsc::sync_channel::<i32>(0);
    cir_trace::ev(&cir_trace::tag_str(), "L1"); let s=thread::spawn(move||{cir_trace::set_tag("tL1");  cir_trace::ev(&cir_trace::tag_str(), "L2"); tx1.send(1).unwrap(); });
    cir_trace::ev(&cir_trace::tag_str(), "L3"); let r=thread::spawn(move||{cir_trace::set_tag("tL3");  cir_trace::ev(&cir_trace::tag_str(), "L4"); rx1.recv().unwrap(); });
    cir_trace::ev(&cir_trace::tag_str(), "L5"); s.join().unwrap(); cir_trace::ev(&cir_trace::tag_str(), "L6"); r.join().unwrap();
    println!("DONE done=1");
cir_trace::finish(); }
