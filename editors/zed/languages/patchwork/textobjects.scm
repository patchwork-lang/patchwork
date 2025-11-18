; Function-like declarations (workers, tasks, skills, functions)
(worker_declaration
  body: (block
    "{" @_start
    "}" @_end
    (#make-range! "function.inside" @_start @_end))) @function.around

(task_declaration
  body: (block
    "{" @_start
    "}" @_end
    (#make-range! "function.inside" @_start @_end))) @function.around

(skill_declaration
  body: (block
    "{" @_start
    "}" @_end
    (#make-range! "function.inside" @_start @_end))) @function.around

(function_declaration
  body: (block
    "{" @_start
    "}" @_end
    (#make-range! "function.inside" @_start @_end))) @function.around

; Class-like declarations (traits, type declarations)
(trait_declaration
  body: (trait_body
    "{" @_start
    "}" @_end
    (#make-range! "class.inside" @_start @_end))) @class.around

(type_declaration) @class.around

; Comments
(comment)+ @comment.around
