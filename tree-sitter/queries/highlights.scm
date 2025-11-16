; Keywords
[
  "worker"
  "trait"
  "task"
  "fun"
  "type"
  "var"
  "if"
  "else"
  "for"
  "while"
  "await"
  "return"
  "succeed"
  "fail"
  "break"
  "continue"
  "import"
  "from"
  "export"
  "think"
  "ask"
] @keyword

((prompt_do) @keyword)

((annotation name: (identifier) @attribute)
 (#match? @attribute "^@?"))

((worker_declaration name: (identifier) @type))
((trait_declaration name: (identifier) @type))
((task_declaration name: (identifier) @function))
((function_declaration name: (identifier) @function))
((type_declaration name: (identifier) @type))

((parameter name: (identifier) @variable.parameter))
((identifier) @variable)

((number) @number)
((boolean) @constant)
((string) @string)
((prompt_text) @string.special)
((prompt_escape) @string.special)
((prompt_interpolation "${" @punctuation.special))
((prompt_interpolation "}" @punctuation.special))

((comment) @comment)

((shell_command_statement "$" @punctuation.special))
((shell_command_statement command: (shell_text) @string.special))
