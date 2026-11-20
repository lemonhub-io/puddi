struct Body {
  mut x : Double
  mut y : Double
  mut z : Double
  mut vx : Double
  mut vy : Double
  mut vz : Double
  mass : Double
}

let solar_mass : Double = 39.47841760435743
let days : Double = 365.24

fn body(x : Double, y : Double, z : Double, vx : Double, vy : Double, vz : Double, m : Double) -> Body {
  { x, y, z, vx: vx * days, vy: vy * days, vz: vz * days, mass: m * solar_mass }
}

fn energy(bs : Array[Body]) -> Double {
  let mut e = 0.0
  let n = bs.length()
  for i = 0; i < n; i = i + 1 {
    let b = bs[i]
    e += 0.5 * b.mass * (b.vx * b.vx + b.vy * b.vy + b.vz * b.vz)
    for j = i + 1; j < n; j = j + 1 {
      let c = bs[j]
      let dx = b.x - c.x
      let dy = b.y - c.y
      let dz = b.z - c.z
      e -= b.mass * c.mass / (dx * dx + dy * dy + dz * dz).sqrt()
    }
  }
  e
}

fn advance(bs : Array[Body], dt : Double) -> Unit {
  let n = bs.length()
  for i = 0; i < n; i = i + 1 {
    let b = bs[i]
    for j = i + 1; j < n; j = j + 1 {
      let c = bs[j]
      let dx = b.x - c.x
      let dy = b.y - c.y
      let dz = b.z - c.z
      let d2 = dx * dx + dy * dy + dz * dz
      let mag = dt / (d2 * d2.sqrt())
      b.vx -= dx * c.mass * mag
      b.vy -= dy * c.mass * mag
      b.vz -= dz * c.mass * mag
      c.vx += dx * b.mass * mag
      c.vy += dy * b.mass * mag
      c.vz += dz * b.mass * mag
    }
  }
  for b in bs {
    b.x += dt * b.vx
    b.y += dt * b.vy
    b.z += dt * b.vz
  }
}

fn main {
  let t = @env.now()
  let bs = [
    body(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0),
    body(4.84143144246472090e+00, -1.16032004402742839e+00, -1.03622044471123109e-01, 1.66007664274403694e-03, 7.69901118419740425e-03, -6.90460016972063023e-05, 9.54791938424326609e-04),
    body(8.34336671824457987e+00, 4.12479856412430479e+00, -4.03523417114321381e-01, -2.76742510726862411e-03, 4.99852801234917238e-03, 2.30417297573763929e-05, 2.85885980666130812e-04),
    body(1.28943695621391310e+01, -1.51111514016986312e+01, -2.23307578892655734e-01, 2.96460137564761618e-03, 2.37847173959480950e-03, -2.96589568540237556e-05, 4.36624404335156298e-05),
    body(1.53796971148509165e+01, -2.59193146099879641e+01, 1.79258772950371181e-01, 2.68067772490389322e-03, 1.62824170038242295e-03, -9.51592254519715870e-05, 5.15138902046611451e-05),
  ]
  let mut px = 0.0
  let mut py = 0.0
  let mut pz = 0.0
  for b in bs {
    px += b.vx * b.mass
    py += b.vy * b.mass
    pz += b.vz * b.mass
  }
  let sun = bs[0]
  sun.vx = -px / solar_mass
  sun.vy = -py / solar_mass
  sun.vz = -pz / solar_mass
  let e0 = energy(bs)
  for s = 0; s < 5_000_000; s = s + 1 {
    advance(bs, 0.01)
  }
  let e1 = energy(bs)
  println("nbody \{(e0 * 1.0e9).round()}/\{(e1 * 1.0e9).round()} \{(@env.now() - t).to_int()}")
}
