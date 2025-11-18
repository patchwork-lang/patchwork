#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <tree_sitter/parser.h>
#include <wctype.h>

#define DEBUG_SCANNER 0

#if DEBUG_SCANNER
#include <stdio.h>
#define DEBUG_LOG(...) fprintf(stderr, __VA_ARGS__)
#else
#define DEBUG_LOG(...)
#endif

#define PROMPT_STACK_CAPACITY 64

enum TokenType {
  PROMPT_START,
  PROMPT_END,
  PROMPT_TEXT,
  PROMPT_ESCAPE,
  PROMPT_INTERPOLATION_START,
  PROMPT_INTERPOLATION_END,
  PROMPT_DO,
  STATEMENT_TERMINATOR,
};

typedef struct {
  uint16_t prompt_depths[PROMPT_STACK_CAPACITY];
  uint8_t prompt_depth_count;
  uint8_t interpolation_depth;
} Scanner;

static inline void reset_state(Scanner *scanner) {
  scanner->prompt_depth_count = 0;
  scanner->interpolation_depth = 0;
}

static inline uint16_t *current_prompt_depth(Scanner *scanner) {
  if (scanner->prompt_depth_count == 0) {
    return NULL;
  }
  return &scanner->prompt_depths[scanner->prompt_depth_count - 1];
}

static inline void push_prompt(Scanner *scanner) {
  if (scanner->prompt_depth_count < PROMPT_STACK_CAPACITY) {
    scanner->prompt_depths[scanner->prompt_depth_count++] = 1;
  }
}

static inline void pop_prompt(Scanner *scanner) {
  if (scanner->prompt_depth_count > 0) {
    scanner->prompt_depth_count--;
  }
}

static inline bool is_whitespace(int32_t c) {
  return c == ' ' || c == '\t' || c == '\r' || c == '\n' || c == '\f';
}

static inline bool is_identifier_continue(int32_t c) {
  return iswalnum(c) || c == '_';
}

static bool scan_statement_terminator(Scanner *scanner, TSLexer *lexer) {
  if (scanner->prompt_depth_count > 0) {
    return false;
  }

  bool saw_newline = false;
  while (true) {
    if (lexer->lookahead == '\r') {
      saw_newline = true;
      lexer->advance(lexer, false);
      if (lexer->lookahead == '\n') {
        lexer->advance(lexer, false);
      }
    } else if (lexer->lookahead == '\n') {
      saw_newline = true;
      lexer->advance(lexer, false);
    } else {
      break;
    }
  }

  if (!saw_newline) {
    return false;
  }

  lexer->result_symbol = STATEMENT_TERMINATOR;
  return true;
}

static bool scan_prompt_start(Scanner *scanner, TSLexer *lexer) {
  if (scanner->prompt_depth_count > 0) {
    return false;
  }

  while (lexer->lookahead == ' ' || lexer->lookahead == '\t' ||
         lexer->lookahead == '\f') {
    lexer->advance(lexer, true);
  }

  DEBUG_LOG("scan_prompt_start char=%d depth_count=%u\n", lexer->lookahead,
            scanner->prompt_depth_count);

  if (lexer->lookahead != '{') {
    return false;
  }

  push_prompt(scanner);
  lexer->advance(lexer, false);
  lexer->result_symbol = PROMPT_START;
  return true;
}

static bool scan_prompt_end(Scanner *scanner, TSLexer *lexer) {
  uint16_t *depth = current_prompt_depth(scanner);
  if (!depth || *depth != 1 || lexer->lookahead != '}') {
    return false;
  }

  pop_prompt(scanner);
  lexer->advance(lexer, false);
  lexer->result_symbol = PROMPT_END;
  return true;
}

static bool scan_prompt_escape(Scanner *scanner, TSLexer *lexer) {
  if (scanner->prompt_depth_count == 0 || lexer->lookahead != '$') {
    return false;
  }

  lexer->advance(lexer, false);
  if (lexer->lookahead != '\'') {
    return false;
  }
  lexer->advance(lexer, false);

  if (lexer->lookahead == 0) {
    return false;
  }
  lexer->advance(lexer, false);

  if (lexer->lookahead != '\'') {
    return false;
  }

  lexer->advance(lexer, false);
  lexer->mark_end(lexer);
  lexer->result_symbol = PROMPT_ESCAPE;
  return true;
}

static bool scan_prompt_interpolation_start(Scanner *scanner, TSLexer *lexer) {
  if (scanner->prompt_depth_count == 0 || lexer->lookahead != '$') {
    return false;
  }

  lexer->advance(lexer, false);
  if (lexer->lookahead != '{') {
    return false;
  }

  lexer->advance(lexer, false);
  lexer->mark_end(lexer);
  scanner->interpolation_depth++;
  lexer->result_symbol = PROMPT_INTERPOLATION_START;
  return true;
}

static bool scan_prompt_interpolation_end(Scanner *scanner, TSLexer *lexer) {
  if (scanner->interpolation_depth == 0 || lexer->lookahead != '}') {
    return false;
  }

  scanner->interpolation_depth--;
  lexer->advance(lexer, false);
  lexer->mark_end(lexer);
  lexer->result_symbol = PROMPT_INTERPOLATION_END;
  return true;
}

static bool scan_prompt_do(Scanner *scanner, TSLexer *lexer) {
  if (scanner->prompt_depth_count == 0 || lexer->lookahead != 'd') {
    return false;
  }

  lexer->advance(lexer, false);
  if (lexer->lookahead != 'o') {
    return false;
  }

  lexer->advance(lexer, false);
  lexer->mark_end(lexer);

  if (is_identifier_continue(lexer->lookahead)) {
    return false;
  }

  while (is_whitespace(lexer->lookahead)) {
    lexer->advance(lexer, true);
  }

  if (lexer->lookahead != '{') {
    return false;
  }

  lexer->result_symbol = PROMPT_DO;
  return true;
}

static bool scan_prompt_text(Scanner *scanner, TSLexer *lexer) {
  uint16_t *depth = current_prompt_depth(scanner);
  if (!depth) {
    DEBUG_LOG("scan_prompt_text missing depth\n");
    return false;
  }

  bool has_content = false;

  DEBUG_LOG("scan_prompt_text start depth=%u char=%d\n", *depth,
            lexer->lookahead);

  for (;;) {
    int32_t c = lexer->lookahead;
    if (c == 0) {
      DEBUG_LOG("prompt_text hit eof\n");
      break;
    }

    if (c == '$' || c == '}') {
      DEBUG_LOG("prompt_text stop char=%d depth=%u\n", c, *depth);
      break;
    }

    // Detect "do" starting a prompt do-block so we can hand control back to the parser.
    if (c == 'd') {
      lexer->advance(lexer, false);  // consume 'd'
      if (lexer->lookahead == 'o') {
        lexer->advance(lexer, false);  // consume 'o'
        while (is_whitespace(lexer->lookahead)) {
          lexer->advance(lexer, true);
        }
        if (lexer->lookahead == '{') {
          lexer->result_symbol = PROMPT_DO;
          DEBUG_LOG("prompt_text emit PROMPT_DO depth=%u\n", *depth);
          return true;
        }
      }
      // Not a do-block; treat consumed chars as text and continue.
      lexer->mark_end(lexer);
      has_content = true;
      continue;
    }

    if (c == '{') {
      (*depth)++;
      lexer->advance(lexer, false);
      lexer->mark_end(lexer);
      has_content = true;
      continue;
    }

    if (c == '}' && *depth > 1) {
      (*depth)--;
      lexer->advance(lexer, false);
      lexer->mark_end(lexer);
      has_content = true;
      continue;
    }

    lexer->advance(lexer, false);
    lexer->mark_end(lexer);
    has_content = true;
  }

  if (has_content) {
    lexer->result_symbol = PROMPT_TEXT;
    DEBUG_LOG("prompt_text emit depth=%u\n", *depth);
    return true;
  }

  DEBUG_LOG("prompt_text no content\n");
  return false;
}

void *tree_sitter_patchwork_external_scanner_create(void) {
  Scanner *scanner = calloc(1, sizeof(Scanner));
  return scanner;
}

void tree_sitter_patchwork_external_scanner_destroy(void *payload) {
  free(payload);
}

void tree_sitter_patchwork_external_scanner_reset(void *payload) {
  Scanner *scanner = (Scanner *)payload;
  if (scanner) {
    reset_state(scanner);
  }
}

unsigned tree_sitter_patchwork_external_scanner_serialize(
  void *payload,
  char *buffer
) {
  Scanner *scanner = (Scanner *)payload;
  if (!scanner || !buffer) {
    return 0;
  }

  unsigned size = 0;
  buffer[size++] = (char)scanner->prompt_depth_count;
  buffer[size++] = (char)scanner->interpolation_depth;
  for (uint8_t i = 0; i < scanner->prompt_depth_count; i++) {
    uint16_t depth = scanner->prompt_depths[i];
    buffer[size++] = (char)(depth & 0xFFu);
    buffer[size++] = (char)((depth >> 8) & 0xFFu);
  }
  return size;
}

void tree_sitter_patchwork_external_scanner_deserialize(
  void *payload,
  const char *buffer,
  unsigned length
) {
  Scanner *scanner = (Scanner *)payload;
  reset_state(scanner);
  if (!buffer || length == 0) {
    return;
  }

  scanner->prompt_depth_count = buffer[0];
  unsigned cursor = 1;
  if (cursor < length) {
    scanner->interpolation_depth = (uint8_t)buffer[cursor++];
  }
  for (uint8_t i = 0; i < scanner->prompt_depth_count; i++) {
    if (cursor + 1 >= length) {
      scanner->prompt_depth_count = i;
      break;
    }
    uint8_t lo = (uint8_t)buffer[cursor++];
    uint8_t hi = (uint8_t)buffer[cursor++];
    scanner->prompt_depths[i] = (uint16_t)(lo | (hi << 8));
  }
}

bool tree_sitter_patchwork_external_scanner_scan(
  void *payload,
  TSLexer *lexer,
  const bool *valid_symbols
) {
  Scanner *scanner = (Scanner *)payload;
  if (!scanner) {
    return false;
  }

  if (valid_symbols[PROMPT_START] && scan_prompt_start(scanner, lexer)) {
    return true;
  }

  if (valid_symbols[PROMPT_INTERPOLATION_END] &&
      scan_prompt_interpolation_end(scanner, lexer)) {
    return true;
  }

  if (valid_symbols[PROMPT_END] && scan_prompt_end(scanner, lexer)) {
    return true;
  }

  if (valid_symbols[PROMPT_INTERPOLATION_START] &&
      scan_prompt_interpolation_start(scanner, lexer)) {
    return true;
  }

  if (valid_symbols[PROMPT_ESCAPE] && scan_prompt_escape(scanner, lexer)) {
    return true;
  }

  if (valid_symbols[STATEMENT_TERMINATOR] &&
      scan_statement_terminator(scanner, lexer)) {
    return true;
  }

  if (valid_symbols[PROMPT_DO] && scan_prompt_do(scanner, lexer)) {
    return true;
  }

  if (valid_symbols[PROMPT_TEXT]) {
    return scan_prompt_text(scanner, lexer);
  }

  return false;
}
