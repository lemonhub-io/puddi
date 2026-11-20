use std::time::Instant;
fn main() {
    let t = Instant::now();
    let size = std::hint::black_box(2000);
    let mut inside = 0;
    for py in 0..size {
        let ci = 2.0 * py as f64 / size as f64 - 1.0;
        for px in 0..size {
            let cr = 2.0 * px as f64 / size as f64 - 1.5;
            let (mut zr, mut zi, mut k) = (0.0f64, 0.0f64, 0);
            while k < 50 && zr * zr + zi * zi <= 4.0 {
                let nr = zr * zr - zi * zi + cr;
                zi = 2.0 * zr * zi + ci;
                zr = nr;
                k += 1;
            }
            if k == 50 { inside += 1; }
        }
    }
    println!("mandel {inside} {}", t.elapsed().as_secs_f64() * 1000.0);
}
