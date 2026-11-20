enum Tree {
  Leaf
  Node(Tree, Tree)
}

fn make(d : Int) -> Tree {
  if d == 0 { Node(Leaf, Leaf) } else { Node(make(d - 1), make(d - 1)) }
}

fn check(t : Tree) -> Int {
  match t {
    Leaf => 0
    Node(l, r) => 1 + check(l) + check(r)
  }
}

fn main {
  let t = @env.now()
  let max_d = 18
  let mut total = check(make(max_d + 1))
  let long_lived = make(max_d)
  let mut d = 4
  while d <= max_d {
    let iters = 1 << (max_d - d + 4)
    let mut sum = 0
    for i = 0; i < iters; i = i + 1 {
      sum += check(make(d))
    }
    total += sum
    d += 2
  }
  total += check(long_lived)
  println("trees \{total} \{(@env.now() - t).to_int()}")
}
