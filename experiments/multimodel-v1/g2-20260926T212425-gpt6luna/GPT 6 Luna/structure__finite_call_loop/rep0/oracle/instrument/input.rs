use concir_sync::Semaphore;

fn helper() {}

fn main() {
    let _ = std::mem::size_of::<Option<Semaphore>>();
    helper();
    helper();
    println!("DONE done=1");
}
