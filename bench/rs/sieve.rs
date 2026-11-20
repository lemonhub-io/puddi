use std::time::Instant;
fn main() {
    let t = Instant::now();
    let n: usize = std::hint::black_box(10_000_000);
    let mut flags = vec![true; n + 1];
    flags[0] = false;
    flags[1] = false;
    let mut i = 2;
    while i * i <= n {
        if flags[i] {
            let mut j = i * i;
            while j <= n { flags[j] = false; j += i; }
        }
        i += 1;
    }
    let count = flags.iter().filter(|&&f| f).count();
    println!("sieve {count} {}", t.elapsed().as_secs_f64() * 1000.0);
}
