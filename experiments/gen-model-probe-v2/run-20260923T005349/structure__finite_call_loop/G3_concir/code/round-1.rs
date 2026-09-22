fn helper() {
    let x: i32 = 1;
    let _ = x;
}

fn main() {
    helper();
    helper();
    let done: i32 = 1;
    println!("DONE done={}", done);
}
