fn main {
  let t = @env.now()
  let n = 500_000
  let sb = StringBuilder()
  for i = 0; i < n; i = i + 1 {
    sb.write_string("w\{i * 7 % 5000} ")
  }
  let text = sb.to_string()
  let counts : Map[String, Int] = Map([])
  for w in text.split(" ") {
    if !w.is_empty() {
      let k = w.to_owned()
      counts[k] = counts.get(k).unwrap_or(0) + 1
    }
  }
  println("strings \{text.length()}/\{counts.length()}/\{counts["w42"]} \{(@env.now() - t).to_int()}")
}
