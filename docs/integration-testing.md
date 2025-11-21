# Integration Testing Framework

## Overview

This document describes the integration testing framework for validating that Patchwork compiler output can be successfully loaded and executed by Claude Code.

## Design Goals

1. **Automated end-to-end validation** - Compile `.pw` source to Claude Code plugins and verify they execute
2. **Isolated test environments** - Each test runs in its own temporary workspace with local plugin marketplace
3. **Non-interactive execution** - Tests run headlessly using `claude -p` for CI/CD compatibility
4. **Simple pass/fail criteria** - Initially validate exit code 0; content validation can be added later

## Test Structure

```
tests/integration/
  greeter/
    source.pw                    # Patchwork source code
    expected_manifest.json       # Expected compiler output (for validation)
    test.sh                      # Test-specific runner (optional overrides)
  <other-tests>/
    ...
```

## Test Execution Flow

### 1. Environment Setup

For each test, create an isolated temporary workspace with the correct directory structure:

```
/tmp/patchwork-test-{random}/
  .claude/
    settings.json                          # Plugin configuration
  marketplace/
    .claude-plugin/
      marketplace.json                     # Marketplace catalog
    plugins/
      greeter/                             # Compiled plugin output
        .claude-plugin/plugin.json
        index.js
        code-process-init.js
        patchwork-runtime.js
        skills/
          greet/SKILL.md
```

**Critical structure requirements:**
- Marketplace must have `.claude-plugin/marketplace.json` catalog
- Plugins must be in `marketplace/plugins/` subdirectory
- Each plugin entry in `marketplace.json` has `source: "./plugins/<name>"` relative to marketplace root

### 2. Marketplace Configuration

Generate `marketplace/.claude-plugin/marketplace.json`:

```json
{
  "name": "test-marketplace",
  "owner": {
    "name": "Patchwork Test Suite"
  },
  "plugins": [
    {
      "name": "greeter",
      "source": "./plugins/greeter",
      "description": "greeter integration test plugin",
      "version": "0.1.0",
      "author": {
        "name": "Patchwork Compiler"
      }
    }
  ]
}
```

**Key points:**
- `source` is relative to marketplace directory (not workspace root)
- Plugin `name` must match directory name in `plugins/`
- Each plugin requires complete metadata

### 3. Settings Configuration

Generate `.claude/settings.json` with nested source structure:

```json
{
  "extraKnownMarketplaces": {
    "test-marketplace": {
      "source": {
        "source": "directory",
        "path": "./marketplace"
      }
    }
  },
  "enabledPlugins": {
    "greeter@test-marketplace": true
  }
}
```

**Key points:**
- `extraKnownMarketplaces` requires nested `source` object with `source` and `path` fields
- `path` is relative to workspace root (MUST be relative, not absolute)
- `enabledPlugins` uses format `<plugin-name>@<marketplace-name>`
- Plugin name matches the `name` field in both `marketplace.json` and `plugin.json`

### 4. Compilation

Compile the test's `.pw` source to the marketplace plugins directory:

```bash
patchworkc tests/integration/greeter/source.pw \
  -o /tmp/patchwork-test-{random}/marketplace/plugins/greeter/
```

### 5. Execution

Run Claude Code non-interactively from the workspace root:

```bash
cd /tmp/patchwork-test-{random}
claude -p "Use the greeter:greet skill to greet Alice" \
  --output-format json \
  --dangerously-skip-permissions
```

**Execution parameters:**
- `-p` - Non-interactive mode (query and exit)
- `--output-format json` - Structured output (for validation)
- `--dangerously-skip-permissions` - Skip permission prompts in test mode
- Working directory = workspace root (so `./marketplace` path resolves correctly)

**Success criteria:**
- Exit code = 0
- JSON output is valid
- Plugin loads and skill executes

**Future enhancements:**
- Parse JSON output and validate response content
- Check for specific patterns in greeting
- Validate tool usage patterns
- Test error conditions and failure modes

### 6. Validation

Current validation steps:

1. **Compilation success** - `patchworkc` exits with code 0
2. **Manifest validation** - Compare generated `.claude-plugin/plugin.json` with `expected_manifest.json`
3. **Marketplace structure** - Verify `marketplace.json` is correctly generated
4. **Execution success** - `claude` exits with code 0
5. **Output format** - JSON output can be parsed (validates `--output-format json` worked)
6. **Plugin loading** - Check that Claude Code discovers and loads the plugin from marketplace

### 7. Cleanup

Remove temporary workspace after test completes (pass or fail). Use `KEEP_WORKSPACE=1` environment variable to preserve workspace for debugging.

## Test Runner Script

Location: `tests/integration/run_test.sh`

Usage:
```bash
./tests/integration/run_test.sh greeter
./tests/integration/run_test.sh --all
```

The script will:
1. Create temporary workspace with unique name
2. Generate settings.json
3. Compile test source
4. Validate manifest output
5. Execute Claude Code with test prompt
6. Report pass/fail
7. Cleanup temporary workspace

## Environment Requirements

- `patchworkc` available in `PATH` or via `cargo run`
- `claude` CLI installed and authenticated
- `/tmp` writable for temporary workspaces

## Example Test Case: Greeter Plugin

**Source:** `tests/integration/greeter/source.pw`

```patchwork
worker greeter_worker(name: string) {
    $ echo "Preparing greeting for ${name}..."

    think {
        You are a friendly AI assistant testing the Patchwork compiler.

        Please greet the user named ${name} with a warm, personalized message.
        Include their name in the greeting and wish them a wonderful day.
    }
}

export default trait Greeter: Agent {
    # Greets a user with a personalized message
    @skill greet
    fun greet(name: string) {
        self.delegate([greeter_worker(name)]).await
    }
}
```

**Invocation:**
```bash
claude -p "Use the greet skill to greet Alice"
```

**Expected behavior:**
- Plugin loads successfully
- Skill is discovered and invoked
- Worker executes shell command and think block
- IPC communication succeeds
- Exit code 0

## Future Enhancements

1. **Content validation** - Parse JSON responses and check for expected patterns
2. **Multi-worker tests** - Test delegation and mailbox communication
3. **Error condition tests** - Verify proper error handling and propagation
4. **Performance benchmarks** - Track compilation and execution time
5. **Concurrent execution** - Test multiple plugins in same workspace
6. **Session state tests** - Test session continuation with `-c` flag

## Key Learnings

### Directory-Based Marketplace Discovery

Through testing, we discovered the exact requirements for Claude Code to discover plugins from a local directory marketplace:

1. **Nested source structure**: The `extraKnownMarketplaces` configuration requires a nested `source` object:
   ```json
   "extraKnownMarketplaces": {
     "test-marketplace": {
       "source": {
         "source": "directory",
         "path": "./marketplace"
       }
     }
   }
   ```

2. **Relative paths are mandatory**: Both `settings.json` paths and `marketplace.json` source paths must be relative, not absolute.

3. **Marketplace catalog required**: A `.claude-plugin/marketplace.json` file must exist at the marketplace root with complete plugin metadata.

4. **Plugin naming consistency**: The plugin name must be consistent across:
   - Directory name in `marketplace/plugins/<name>/`
   - `name` field in `marketplace.json` plugin entry
   - `name` field in `.claude-plugin/plugin.json`
   - `enabledPlugins` key as `<name>@<marketplace>`

5. **Non-interactive mode**: Local marketplace plugins can be loaded in `-p` mode with proper configuration, enabling automated integration testing.

## Implementation Plan

1. ✅ Create `tests/integration/run_test.sh` runner script
2. ✅ Move `/tmp/greeter-plugin.pw` to `tests/integration/greeter/source.pw`
3. ✅ Create `expected_manifest.json` from current compiler output
4. ✅ Validate test passes with current implementation
5. Add more test cases as needed (multi-worker, error handling, etc.)

## Related Documents

- [CLI Reference](https://code.claude.com/docs/en/cli-reference) - Claude Code command-line options
- [Settings Documentation](https://code.claude.com/docs/en/settings) - Configuration file structure
- Phase 12 implementation notes - Runtime IPC architecture
