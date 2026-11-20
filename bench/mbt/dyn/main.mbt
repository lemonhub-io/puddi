trait Shape {
  area(Self) -> Double
}

struct Sq { s : Double }
struct Rect { w : Double; h : Double }
struct Tri { b : Double; h : Double }

impl Shape for Sq with area(self) { self.s * self.s }
impl Shape for Rect with area(self) { self.w * self.h }
impl Shape for Tri with area(self) { 0.5 * self.b * self.h }

fn main {
  let t = @env.now()
  let shapes : Array[&Shape] = []
  for i = 0; i < 1000; i = i + 1 {
    let f = (i % 10).to_double()
    if i % 3 == 0 {
      shapes.push(Sq::{ s: f })
    } else if i % 3 == 1 {
      shapes.push(Rect::{ w: f, h: 2.0 })
    } else {
      shapes.push(Tri::{ b: f, h: 3.0 })
    }
  }
  let mut total = 0.0
  for r = 0; r < 20_000; r = r + 1 {
    for sh in shapes { total += sh.area() }
  }
  println("dyn \{total} \{(@env.now() - t).to_int()}")
}
