use std::time::Instant;
trait Shape { fn area(&self) -> f64; }
struct Sq { s: f64 }
struct Rect { w: f64, h: f64 }
struct Tri { b: f64, h: f64 }
impl Shape for Sq { fn area(&self) -> f64 { self.s * self.s } }
impl Shape for Rect { fn area(&self) -> f64 { self.w * self.h } }
impl Shape for Tri { fn area(&self) -> f64 { 0.5 * self.b * self.h } }
fn main() {
    let t = Instant::now();
    let mut shapes: Vec<Box<dyn Shape>> = Vec::new();
    for i in 0..std::hint::black_box(1000) {
        let f = (i % 10) as f64;
        match i % 3 { 0 => shapes.push(Box::new(Sq { s: f })), 1 => shapes.push(Box::new(Rect { w: f, h: 2.0 })), _ => shapes.push(Box::new(Tri { b: f, h: 3.0 })) }
    }
    let mut total = 0.0;
    for _ in 0..std::hint::black_box(20_000) { for sh in &shapes { total += sh.area(); } }
    println!("dyn {total} {}", t.elapsed().as_secs_f64() * 1000.0);
}
