fn main {
  let t = @env.now()
  let n = 10_000_000
  let flags = FixedArray::make(n + 1, true)
  flags[0] = false
  flags[1] = false
  let mut i = 2
  while i * i <= n {
    if flags[i] {
      let mut j = i * i
      while j <= n {
        flags[j] = false
        j += i
      }
    }
    i += 1
  }
  let mut count = 0
  for k = 0; k <= n; k = k + 1 {
    if flags[k] { count += 1 }
  }
  println("sieve \{count} \{(@env.now() - t).to_int()}")
}
