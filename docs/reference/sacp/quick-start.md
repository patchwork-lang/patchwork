# How to test out Symposium ACP

## Install the Symposium ACP conductor

```bash
cargo install sacp-conductor
```

## Configure Zed

Type:

* `Command-P` / open settings file

Add JSON like this:

```json
{
  "agent_servers": {
    "Sparkle": {
      "default_mode": "bypassPermissions",
      "command": "/Users/nikomat/.cargo/bin/sacp-conductor",
      "args": [
        "agent",
        "npx -y '@zed-industries/claude-code-acp'"
      ],
      "env": {}
    }
  },
  ...
}
```

Click "+" in the agents area and pick Sparkle:



Explaining the lines

* `sacp-conductor agent A B ... Z` will
    * run `A`..`Y` as proxies
    * run `Z` as an agent
* in this case there is just the agent
    * `npx -y '@zed-industries/claude-code-acp'` is the agent

## Add a proxy

Let's try Sparkle as an example

```bash
cargo install sparkle-mcp
```

Modify configuration to


```json
{
  "agent_servers": {
    "Sparkle": {
      "default_mode": "bypassPermissions",
      "command": "/Users/nikomat/.cargo/bin/sacp-conductor",
      "args": [
        "agent",
        "/Users/nikomat/.cargo/bin/sparkle-mcp --acp",
        "npx -y '@zed-industries/claude-code-acp'"
      ],
      "env": {}
    }
  },
  ...
}
```

## Writing a proxy

Here is a minimal proxy example

https://github.com/symposium-dev/symposium-acp/blob/main/src/sacp-proxy/examples/minimal.rs

Here is the Sparkle ACP proxy

https://github.com/symposium-dev/sparkle/blob/main/sparkle-mcp/src/acp_component.rs
