; Inject Markdown into prompt blocks (think/ask)
((prompt_text) @injection.content
 (#set! injection.language "markdown"))

; Inject Patchwork code into prompt do blocks
((prompt_do_block
  (block) @injection.content)
 (#set! injection.language "patchwork"))

; Inject shell/bash into shell command statements
((shell_command_statement
  command: (shell_text) @injection.content)
 (#set! injection.language "bash"))

; Inject shell/bash into shell command expressions
((shell_command_expression
  command: (shell_inner_text) @injection.content)
 (#set! injection.language "bash"))

; Comments can contain markdown-style documentation
((comment) @injection.content
 (#match? @injection.content "^#[#!]")
 (#set! injection.language "markdown"))
