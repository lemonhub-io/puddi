use std::time::Instant;
enum Tree { Leaf, Node(Box<Tree>, Box<Tree>) }
fn make(d: i32) -> Tree {
    if d == 0 { Tree::Node(Box::new(Tree::Leaf), Box::new(Tree::Leaf)) } else { Tree::Node(Box::new(make(d - 1)), Box::new(make(d - 1))) }
}
fn check(t: &Tree) -> i32 { match t { Tree::Leaf => 0, Tree::Node(l, r) => 1 + check(l) + check(r) } }
fn main() {
    let t = Instant::now();
    let max_d = std::hint::black_box(18);
    let mut total = check(&make(max_d + 1));
    let long_lived = make(max_d);
    let mut d = 4;
    while d <= max_d {
        let iters = 1 << (max_d - d + 4);
        let mut sum = 0;
        for _ in 0..iters { sum += check(&make(d)); }
        total += sum;
        d += 2;
    }
    total += check(&long_lived);
    println!("trees {total} {}", t.elapsed().as_secs_f64() * 1000.0);
}
