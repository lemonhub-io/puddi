fn main {
  let t = @env.now()
  let n = 1_000_000
  let xs : Array[Int] = Array(capacity=n)
  let mut s : Int64 = 7L
  for i = 0; i < n; i = i + 1 {
    s = (s * 1103515245L + 12345L) % 2147483648L
    xs.push(s.to_int())
  }
  xs.sort()
  let mut ok = true
  for k = 1; k < n; k = k + 1 {
    if xs[k - 1] > xs[k] { ok = false }
  }
  println("sort \{ok}/\{xs[0]}/\{xs[n / 2]}/\{xs[n - 1]} \{(@env.now() - t).to_int()}")
}
