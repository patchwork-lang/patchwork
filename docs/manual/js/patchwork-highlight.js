// Patchwork language definition for highlight.js
// Registers 'patchwork' and 'pw' as language identifiers

(function() {
  function patchwork(hljs) {
    // hljs 10.x expects space-separated strings, not arrays
    const KEYWORDS = {
      keyword:
        'import export from default ' +
        'var fun worker trait skill type ' +
        'if else for while in break ' +
        'return succeed throw ' +
        'think ask do ' +
        'await self',
      literal: 'true false null',
      built_in: 'cat log'
    };

    // Shell command mode (after $ or inside $(...))
    const SHELL_ARG = {
      className: 'string',
      begin: /[^\s\$\"\(\)<>|&=\\]+/
    };

    const SHELL_MODE = {
      className: 'meta',
      begin: /\$\s+/,
      end: /$/,
      contains: [
        {
          className: 'string',
          begin: /"/,
          end: /"/,
          contains: [
            { begin: /\\./ },
            {
              className: 'subst',
              begin: /\$\{/,
              end: /\}/,
              contains: ['self']
            },
            {
              className: 'subst',
              begin: /\$/,
              end: /(?=[^a-zA-Z0-9_]|$)/
            }
          ]
        },
        SHELL_ARG
      ]
    };

    // Command substitution $(...)
    const COMMAND_SUBST = {
      className: 'meta',
      begin: /\$\(/,
      end: /\)/,
      contains: [
        {
          className: 'string',
          begin: /"/,
          end: /"/,
          contains: [{ begin: /\\./ }]
        },
        SHELL_ARG
      ]
    };

    // String interpolation
    const STRING = {
      className: 'string',
      begin: /"/,
      end: /"/,
      contains: [
        { begin: /\\./ },  // escape sequences
        {
          className: 'subst',
          begin: /\$\{/,
          end: /\}/,
          contains: ['self']
        },
        {
          className: 'subst',
          begin: /\$\(/,
          end: /\)/,
          contains: [SHELL_ARG]
        },
        {
          className: 'subst',
          begin: /\$/,
          end: /(?=[^a-zA-Z0-9_]|$)/
        }
      ]
    };

    // Single-quoted strings (no interpolation)
    const SINGLE_STRING = {
      className: 'string',
      begin: /'/,
      end: /'/,
      contains: [{ begin: /\\./ }]
    };

    // Think/ask blocks with prompt content
    // In hljs 10.x, we use returnBegin + contains with the keyword
    const THINK_BLOCK = {
      begin: /\b(think|ask)\s*\{/,
      end: /\}/,
      returnBegin: true,
      contains: [
        {
          className: 'keyword',
          begin: /\b(think|ask)\b/
        },
        {
          className: 'subst',
          begin: /\$\{/,
          end: /\}/,
          contains: ['self']
        },
        {
          className: 'subst',
          begin: /\$\(/,
          end: /\)/
        },
        {
          className: 'subst',
          begin: /\$/,
          end: /(?=[^a-zA-Z0-9_]|$)/
        },
        {
          // Nested do { } blocks return to code mode
          begin: /\bdo\s*\{/,
          end: /\}/,
          returnBegin: true,
          contains: [
            {
              className: 'keyword',
              begin: /\bdo\b/
            },
            'self'
          ]
        }
      ]
    };

    // Decorator/attribute
    const DECORATOR = {
      className: 'meta',
      begin: /@/,
      end: /(?=\s|$|\()/,
      contains: [
        { className: 'keyword', begin: /[a-zA-Z_][a-zA-Z0-9_]*/ }
      ]
    };

    // Comments
    const COMMENT = hljs.COMMENT('#', '$');

    // Numbers
    const NUMBER = {
      className: 'number',
      begin: /\b\d+(\.\d+)?\b/
    };

    // Function/worker/trait definitions
    const DEFINITION = {
      beginKeywords: 'fun worker trait skill',
      end: /[{(]/,
      excludeEnd: true,
      contains: [
        {
          className: 'title.function',
          begin: /[a-zA-Z_][a-zA-Z0-9_]*/
        },
        {
          className: 'params',
          begin: /\(/,
          end: /\)/,
          contains: [
            { className: 'variable', begin: /[a-zA-Z_][a-zA-Z0-9_]*/ },
            { className: 'type', begin: /:\s*/, end: /[,)]/, excludeEnd: true }
          ]
        }
      ]
    };

    // Import statements
    const IMPORT = {
      begin: /\bimport\b/,
      end: /$/,
      beginScope: 'keyword',
      contains: [
        { className: 'string', begin: /\.\/[^\s{]+/ },
        { className: 'string', begin: /std\.[a-zA-Z_][a-zA-Z0-9_]*/ },
        {
          begin: /\{/,
          end: /\}/,
          contains: [
            { className: 'variable', begin: /[a-zA-Z_][a-zA-Z0-9_]*/ }
          ]
        }
      ]
    };

    // Type annotations
    const TYPE = {
      className: 'type',
      begin: /:\s*/,
      end: /(?=[=,)\]{}]|\s)/,
      excludeBegin: true,
      contains: [
        { begin: /[A-Z][a-zA-Z0-9_]*/ },
        { begin: /string|number|boolean|any|void/ }
      ]
    };

    return {
      name: 'Patchwork',
      aliases: ['pw'],
      keywords: KEYWORDS,
      contains: [
        COMMENT,
        THINK_BLOCK,
        SHELL_MODE,
        COMMAND_SUBST,
        STRING,
        SINGLE_STRING,
        DECORATOR,
        NUMBER,
        DEFINITION,
        IMPORT,
        TYPE,
        {
          // Operators
          className: 'operator',
          begin: /->|=>|\.\.\.|\|\||&&|==|!=|<=|>=|\+\+|--/
        }
      ]
    };
  }

  // Register the language and re-highlight patchwork blocks
  if (typeof hljs !== 'undefined') {
    hljs.registerLanguage('patchwork', patchwork);
    hljs.registerLanguage('pw', patchwork);

    // Re-highlight any patchwork code blocks that weren't processed
    // Use highlightBlock for hljs 10.x compatibility (mdbook uses 10.1.1)
    document.querySelectorAll('pre code.language-patchwork, pre code.language-pw').forEach(function(block) {
      hljs.highlightBlock(block);
    });
  }
})();
