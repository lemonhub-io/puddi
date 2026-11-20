use std::time::Instant;
fn main() {
    let t = Instant::now();
    let n = std::hint::black_box(10_000_000);
    let r: i64 = (0..n).map(|x: i32| x % 1000).filter(|x| x % 3 == 0).fold(0i64, |a, x| a + x as i64);
    println!("iter {r} {}", t.elapsed().as_secs_f64() * 1000.0);
}
