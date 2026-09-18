fn release(&self) {
    *self.permits.lock().unwrap() += 1;
    self.cv.notify_one();
}
