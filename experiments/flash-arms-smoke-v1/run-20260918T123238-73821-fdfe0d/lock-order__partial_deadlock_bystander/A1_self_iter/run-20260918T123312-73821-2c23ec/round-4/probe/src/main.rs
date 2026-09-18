let _g1 = m1a.lock().unwrap();
let _g2 = m2a.lock().unwrap();
sem_a.release();
