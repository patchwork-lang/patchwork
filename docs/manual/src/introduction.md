# Introduction

Patchwork is a scripting language designed to make developing AI-powered automation easy. It combines deterministic code execution with LLM reasoning through a unique feature called **think blocks**.

```patchwork
var task = "Review this code for security issues"
var files = $(find src -name "*.rs")

var analysis = think {
    The user wants to: ${task}

    Review these files for common security vulnerabilities:
    ${files}
}

print(analysis)
```

Patchwork uses the [Agent/Client Protocol](https://agentclientprotocol.com/) to bring a shell-like scripting experience directly to popular coding agents.

## What Makes Patchwork Different?

Most LLM integrations treat AI as an API call—you send a prompt, get a response, and that's it. Patchwork treats LLM reasoning as a **first-class language construct**:

- **Think blocks** pause execution, consult an LLM, and return a value
- **Shell integration** as convenient as traditional shells but designed for portability (TODO)
- **Structured agents** can delegate work to sub-agents with defined skills (TODO)
- **Deterministic execution** means your code runs the same way every time—only LLM responses vary
