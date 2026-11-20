fn main {
  let t = @env.now()
  let size = 2000
  let mut inside = 0
  for py = 0; py < size; py = py + 1 {
    let ci = 2.0 * py.to_double() / size.to_double() - 1.0
    for px = 0; px < size; px = px + 1 {
      let cr = 2.0 * px.to_double() / size.to_double() - 1.5
      let mut zr = 0.0
      let mut zi = 0.0
      let mut k = 0
      while k < 50 && zr * zr + zi * zi <= 4.0 {
        let nr = zr * zr - zi * zi + cr
        zi = 2.0 * zr * zi + ci
        zr = nr
        k += 1
      }
      if k == 50 { inside += 1 }
    }
  }
  println("mandel \{inside} \{(@env.now() - t).to_int()}")
}
