use std::time::Instant;
fn fib(n: i32) -> i32 { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }
fn main() {
    let t = Instant::now();
    let r = fib(std::hint::black_box(38));
    println!("fib {r} {}", t.elapsed().as_secs_f64() * 1000.0);
}
