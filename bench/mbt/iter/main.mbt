fn main {
  let t = @env.now()
  let n = 10_000_000
  let r = (0).until(n)
    .map(fn(x) { x % 1000 })
    .filter(fn(x) { x % 3 == 0 })
    .fold(init=0L, fn(a, x) { a + x.to_int64() })
  println("iter \{r} \{(@env.now() - t).to_int()}")
}
