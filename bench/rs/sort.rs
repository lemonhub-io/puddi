use std::time::Instant;
fn main() {
    let t = Instant::now();
    let n = std::hint::black_box(1_000_000);
    let mut xs: Vec<i32> = Vec::with_capacity(n);
    let mut s: i64 = 7;
    for _ in 0..n { s = (s * 1103515245 + 12345) % 2147483648; xs.push(s as i32); }
    xs.sort();
    let ok = xs.windows(2).all(|w| w[0] <= w[1]);
    println!("sort {ok}/{}/{}/{} {}", xs[0], xs[n / 2], xs[n - 1], t.elapsed().as_secs_f64() * 1000.0);
}
