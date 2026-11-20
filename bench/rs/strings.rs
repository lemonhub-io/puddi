use std::collections::HashMap;
use std::fmt::Write;
use std::time::Instant;
fn main() {
    let t = Instant::now();
    let n = std::hint::black_box(500_000);
    let mut text = String::new();
    for i in 0..n { write!(text, "w{} ", i * 7 % 5000).unwrap(); }
    let mut counts: HashMap<&str, i32> = HashMap::new();
    for w in text.split_whitespace() { *counts.entry(w).or_insert(0) += 1; }
    println!("strings {}/{}/{} {}", text.len(), counts.len(), counts["w42"], t.elapsed().as_secs_f64() * 1000.0);
}
