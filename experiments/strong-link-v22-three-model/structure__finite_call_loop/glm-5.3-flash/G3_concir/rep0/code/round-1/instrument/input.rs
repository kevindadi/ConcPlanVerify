fn helper() {}

fn main() {
    let mut done: i32 = 0;
    helper();
    helper();
    done = 1;
    println!("DONE done={}", done);
}
