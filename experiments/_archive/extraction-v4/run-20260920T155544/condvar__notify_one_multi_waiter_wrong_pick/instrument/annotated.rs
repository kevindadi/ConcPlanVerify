mod cir_trace;
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread;

fn main() {
    let p=Arc::new((Mutex::new(false), Condvar::new()));
    let mut hs=vec![];
    for _ in 0..2 { let q=Arc::clone(&p); cir_trace::ev(&cir_trace::tag_str(), "L1"); hs.push(thread::spawn(move||{cir_trace::set_tag("tL1");  let (m,cv)=&*q;
        cir_trace::ev(&cir_trace::tag_str(), "L2"); let mut g=m.lock().unwrap(); while !*g { cir_trace::ev(&cir_trace::tag_str(), "L3"); g=cv.wait(g).unwrap(); } })); }
    let q=Arc::clone(&p); cir_trace::ev(&cir_trace::tag_str(), "L4"); let n=thread::spawn(move||{cir_trace::set_tag("tL4");  let (m,cv)=&*q; cir_trace::ev(&cir_trace::tag_str(), "L5"); let mut g=m.lock().unwrap();
        *g=true; cir_trace::ev(&cir_trace::tag_str(), "L6"); cv.notify_all(); });
    cir_trace::ev(&cir_trace::tag_str(), "L7"); n.join().unwrap(); for h in hs { cir_trace::ev(&cir_trace::tag_str(), "L8"); h.join().unwrap(); }
    println!("DONE waiters=0");
cir_trace::finish(); }
