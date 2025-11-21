#!/bin/bash

set -euo pipefail

# Integration test runner for Patchwork compiler
# Compiles .pw source to Claude Code plugins and validates execution

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
PATCHWORKC="${PATCHWORKC:-cargo run --quiet --bin patchworkc --}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

usage() {
    echo "Usage: $0 [--all] [TEST_NAME]"
    echo ""
    echo "Examples:"
    echo "  $0 greeter          # Run greeter test"
    echo "  $0 --all            # Run all tests"
    echo ""
    echo "Environment variables:"
    echo "  PATCHWORKC          # Path to patchworkc compiler (default: cargo run)"
    exit 1
}

log() {
    echo -e "${GREEN}[TEST]${NC} $*"
}

error() {
    echo -e "${RED}[ERROR]${NC} $*" >&2
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

run_test() {
    local test_name="$1"
    local test_dir="$SCRIPT_DIR/$test_name"

    if [[ ! -d "$test_dir" ]]; then
        error "Test directory not found: $test_dir"
        return 1
    fi

    if [[ ! -f "$test_dir/source.pw" ]]; then
        error "Missing source.pw in $test_name"
        return 1
    fi

    log "Running test: $test_name"

    # Create temporary workspace with correct directory structure
    local workspace
    workspace=$(mktemp -d /tmp/patchwork-test-XXXXXX)
    log "Created workspace: $workspace"

    # Setup cleanup trap (skip if KEEP_WORKSPACE is set)
    if [[ -z "${KEEP_WORKSPACE:-}" ]]; then
        trap "rm -rf '$workspace'" EXIT
    else
        log "KEEP_WORKSPACE set, preserving workspace at $workspace"
    fi

    # Create directory structure:
    # workspace/
    #   .claude/settings.json
    #   marketplace/
    #     .claude-plugin/marketplace.json
    #     plugins/
    #       greeter/
    mkdir -p "$workspace/.claude"
    mkdir -p "$workspace/marketplace/.claude-plugin"
    mkdir -p "$workspace/marketplace/plugins"

    # Determine plugin name (use test name by default)
    local plugin_name="${test_name}"
    local plugin_dir="$workspace/marketplace/plugins/$plugin_name"

    # Compile the plugin to marketplace/plugins/
    log "Compiling $test_name/source.pw -> marketplace/plugins/$plugin_name/"
    if ! $PATCHWORKC "$test_dir/source.pw" -o "$plugin_dir" 2>&1 | sed 's/^/  /'; then
        error "Compilation failed"
        return 1
    fi

    # Validate manifest exists
    if [[ ! -f "$plugin_dir/.claude-plugin/plugin.json" ]]; then
        error "Compilation did not produce .claude-plugin/plugin.json"
        return 1
    fi

    # Validate expected manifest if provided
    if [[ -f "$test_dir/expected_manifest.json" ]]; then
        log "Validating manifest against expectations"
        if ! diff -u "$test_dir/expected_manifest.json" "$plugin_dir/.claude-plugin/plugin.json" | sed 's/^/  /'; then
            warn "Manifest differs from expected (this may be intentional)"
        fi
    fi

    # Generate marketplace.json
    log "Generating marketplace/.claude-plugin/marketplace.json"
    cat > "$workspace/marketplace/.claude-plugin/marketplace.json" <<EOF
{
  "name": "test-marketplace",
  "owner": {
    "name": "Patchwork Test Suite"
  },
  "plugins": [
    {
      "name": "$plugin_name",
      "source": "./plugins/$plugin_name",
      "description": "$test_name integration test plugin",
      "version": "0.1.0",
      "author": {
        "name": "Patchwork Compiler"
      }
    }
  ]
}
EOF

    # Generate settings.json with correct marketplace structure
    log "Generating .claude/settings.json"
    cat > "$workspace/.claude/settings.json" <<EOF
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
    "$plugin_name@test-marketplace": true
  }
}
EOF

    # Determine test prompt
    local test_prompt
    if [[ -f "$test_dir/prompt.txt" ]]; then
        test_prompt=$(<"$test_dir/prompt.txt")
        log "Using custom prompt from prompt.txt"
    else
        # Default prompt based on plugin structure
        test_prompt="Use the skills from $plugin_name to demonstrate its functionality"
        log "Using default prompt"
    fi

    # Execute Claude Code
    log "Executing: claude -p \"$test_prompt\" --output-format json --dangerously-skip-permissions"
    local output_file="$workspace/output.json"

    if ! (cd "$workspace" && claude -p "$test_prompt" --output-format json --dangerously-skip-permissions > "$output_file" 2>&1); then
        error "Claude Code execution failed"
        cat "$output_file" | sed 's/^/  /'
        return 1
    fi

    # Validate JSON output
    if ! jq empty "$output_file" 2>/dev/null; then
        error "Output is not valid JSON"
        cat "$output_file" | sed 's/^/  /'
        return 1
    fi

    log "Output saved to: $output_file"

    # Success!
    echo -e "${GREEN}✓${NC} Test $test_name passed"
    return 0
}

# Main execution

if [[ $# -eq 0 ]]; then
    usage
fi

if [[ "$1" == "--all" ]]; then
    log "Running all integration tests"
    failed_tests=()
    passed_tests=()

    for test_dir in "$SCRIPT_DIR"/*/; do
        test_name=$(basename "$test_dir")

        if run_test "$test_name"; then
            passed_tests+=("$test_name")
        else
            failed_tests+=("$test_name")
        fi
        echo ""
    done

    # Summary
    echo "========================================"
    echo "Test Summary"
    echo "========================================"
    echo "Passed: ${#passed_tests[@]}"
    echo "Failed: ${#failed_tests[@]}"

    if [[ ${#failed_tests[@]} -gt 0 ]]; then
        echo ""
        echo "Failed tests:"
        for test in "${failed_tests[@]}"; do
            echo "  - $test"
        done
        exit 1
    fi

    exit 0
else
    # Run single test
    test_name="$1"
    run_test "$test_name"
fi
