function FnOutputInt(input: string) -> int {
  client GPT35
  prompt #"
    Return the integer 5 with no additional context.
  "#
}

test FnOutputInt {
  functions [FnOutputInt]
  args {
    input "example input"
  }
}
