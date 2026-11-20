fn fib(n : Int) -> Int {
  if n < 2 { n } else { fib(n - 1) + fib(n - 2) }
}

fn main {
  let t = @env.now()
  let r = fib(38)
  println("fib \{r} \{(@env.now() - t).to_int()}")
}
