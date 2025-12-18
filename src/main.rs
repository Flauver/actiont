static mut I: i32 = 0;

fn main() {
    for _ in 0..100 {
        unsafe {
            I += 1;
        }
        println!("{}", unsafe { I })
    }
}
