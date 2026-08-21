the regex: abc|c*d

Alternation(
  Concat(
    Concat(
      Literal(a),
      Literal(c)
    ),
    Literal(b)
  ),
  Concat(
    Star(
      Literal(c)
    ),
    Literal(d)
  )
)
