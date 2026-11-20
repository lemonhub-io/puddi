fn main {
  let t = @env.now()
  let n = 1_000_000
  let m : Map[Int, Int] = Map([])
  let mut s : Int64 = 42L
  for i = 0; i < n; i = i + 1 {
    s = (s * 1103515245L + 12345L) % 2147483648L
    m[(s % 2000000L).to_int()] = i
  }
  let mut hits = 0
  let mut sum : Int64 = 0L
  for q = 0; q < n; q = q + 1 {
    if m.get(q * 2) is Some(v) {
      hits += 1
      sum += v.to_int64()
    }
  }
  println("hashmap \{m.length()}/\{hits}/\{sum} \{(@env.now() - t).to_int()}")
}
