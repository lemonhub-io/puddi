use std::collections::HashMap;
use std::time::Instant;
fn main() {
    let t = Instant::now();
    let n = std::hint::black_box(1_000_000);
    let mut m: HashMap<i32, i32> = HashMap::new();
    let mut s: i64 = 42;
    for i in 0..n {
        s = (s * 1103515245 + 12345) % 2147483648;
        m.insert((s % 2000000) as i32, i);
    }
    let (mut hits, mut sum) = (0, 0i64);
    for q in 0..n {
        if let Some(&v) = m.get(&(q * 2)) { hits += 1; sum += v as i64; }
    }
    println!("hashmap {}/{hits}/{sum} {}", m.len(), t.elapsed().as_secs_f64() * 1000.0);
}
