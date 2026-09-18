sem_ab_b.acquire();       // wait for A to hold m1
let g2 = m2b.lock().unwrap();  // lock m2
drop(g2);                 // unlock m2
sem_ba_b.release();       // signal A
let _g1 = m1b.lock().unwrap(); // lock m1
