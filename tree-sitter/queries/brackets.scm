; Curly braces for blocks and trait bodies
((block "{" @open "}" @close))
((trait_body "{" @open "}" @close))

; Prompt delimiters
((prompt_body
  (prompt_start) @open
  (prompt_end) @close))

; Parentheses
((parenthesized_expression "(" @open ")" @close))
((argument_list "(" @open ")" @close))
((parameter_list "(" @open ")" @close))

; Square brackets
((array_literal "[" @open "]" @close))
