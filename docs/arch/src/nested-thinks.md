# Nested Think Blocks

Think blocks can nest: deterministic code inside a think block might itself contain think blocks. This chapter explains how the system handles this recursive interplay without deadlock.

## When Nesting Occurs

Nested think blocks arise when:

1. A think block includes code fragments the LLM can execute via the `do` tool
2. That code contains another think block
3. The inner think must complete before the outer think can continue

```patchwork
var analysis = think {
    Analyze this code and determine what tests to write.

    You can run this to see the current tests:
    do {
        $ ls tests/
    }

    You can also ask for clarification:
    do {
        var answer = think {
            What testing framework does this project use?
        }
        print(answer)
    }
}
```

Here the outer think might invoke the inner `do` blocks, each of which could trigger further think blocks.

## The Stack-Based Solution

The redirect actor maintains a stack of active think handlers:

```mermaid
graph TD
    subgraph "Redirect Actor State"
        S[Stack]
        S --> T1[Think Handler 1<br/>outer]
        S --> T2[Think Handler 2<br/>inner]
        S --> T3[Think Handler 3<br/>innermost]
    end

    N[Incoming Notification] --> S
    S -->|routes to top| T3
```

When a notification arrives from the successor, it goes to the **top of the stack**—the innermost active think block. This is correct because:

- The innermost think is the one currently waiting for LLM responses
- Outer thinks are blocked, waiting for their `do` invocations to complete
- When the innermost completes, it pops off, and the next one down becomes active

## Execution Flow

Here's what happens when nested think blocks execute:

```mermaid
sequenceDiagram
    participant E as Evaluator Thread
    participant R as Redirect Actor
    participant T1 as Think Handler 1
    participant T2 as Think Handler 2
    participant S as Successor

    Note over E: Outer think block starts
    E->>R: ThinkRequest (outer)
    R->>R: Create T1
    R->>R: Push T1 onto stack

    T1->>S: session/new
    T1->>S: prompt (outer)

    Note over T1: LLM decides to call do(0)
    S-->>R: tool_call: do(0)
    R-->>T1: DoInvocation(0)
    T1->>E: ThinkResponse::Do(0)

    Note over E: Evaluate do block,<br/>hits inner think
    E->>R: ThinkRequest (inner)
    R->>R: Create T2
    R->>R: Push T2 onto stack

    T2->>S: session/new
    T2->>S: prompt (inner)

    rect rgb(200, 230, 200)
        Note over T2,S: Inner think conversation
        S-->>R: notification (chunk)
        R-->>T2: route to top of stack
        S-->>R: PromptResponse
        R-->>T2: complete
    end

    T2->>R: Pop T2
    T2->>E: ThinkResponse::Complete (inner result)

    Note over E: Inner think done,<br/>continue do block
    E->>T1: do(0) result

    rect rgb(200, 200, 230)
        Note over T1,S: Outer think continues
        S-->>R: notification (chunk)
        R-->>T1: route to top of stack
        S-->>R: PromptResponse
        R-->>T1: complete
    end

    T1->>R: Pop T1
    T1->>E: ThinkResponse::Complete (outer result)
```

## Why This Doesn't Deadlock

The key insight is that **different channels are used at each level**:

| Component | Waits On | Sends To |
|-----------|----------|----------|
| Evaluator (outer think) | `rx1` ([std::sync::mpsc](https://doc.rust-lang.org/std/sync/mpsc/index.html)) | Agent via `tx` ([tokio::sync::mpsc](https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html)) |
| Think Handler 1 | `think_rx1` ([tokio::sync::mpsc](https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html)) | Evaluator via `response_tx1` |
| Evaluator (inner think) | `rx2` ([std::sync::mpsc](https://doc.rust-lang.org/std/sync/mpsc/index.html)) | Agent via `tx` ([tokio::sync::mpsc](https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html)) |
| Think Handler 2 | `think_rx2` ([tokio::sync::mpsc](https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html)) | Evaluator via `response_tx2` |

Each think block creates a fresh `response_tx`/`response_rx` pair. The evaluator blocks on its receiver, but the async runtime continues processing. When the inner think completes, it sends on `response_tx2`, unblocking the evaluator, which then sends the result back to the outer think handler.

## The Channel Dance

Let's trace the channels at maximum nesting:

```mermaid
sequenceDiagram
    participant E as Evaluator Thread
    participant A as Agent
    participant T1 as Think Handler 1
    participant T2 as Think Handler 2
    participant S as Successor

    Note over E: eval_think_block (outer)
    E->>A: ThinkRequest via tx
    Note over E: blocked on rx1

    A->>T1: Create handler
    Note over A: Push T1 onto stack
    T1->>S: prompt (outer)

    S-->>T1: do(0) tool call
    T1->>E: ThinkResponse::Do via rx1
    Note over E: Unblocked

    Note over E: eval do(0) block
    Note over T1: Waiting for do result

    Note over E: eval_think_block (inner)
    E->>A: ThinkRequest via tx
    Note over E: blocked on rx2

    A->>T2: Create handler
    Note over A: Push T2 onto stack
    T2->>S: prompt (inner)

    S-->>T2: response complete
    Note over A: Pop T2
    T2->>E: ThinkResponse::Complete via rx2
    Note over E: Unblocked with inner result

    E->>T1: do(0) result
    Note over T1: Continue conversation

    S-->>T1: response complete
    Note over A: Pop T1
    T1->>E: ThinkResponse::Complete via rx1
    Note over E: Unblocked with outer result
```

## Call Stack at Deepest Point

When the inner think is waiting for its LLM response, here's the state of each stack. Stacks grow upward—the top of each stack is the most recently pushed frame:

```mermaid
graph BT
    subgraph EvalThread ["Evaluator Thread Call Stack"]
        direction BT
        E4["eval_think_block (outer)<br/>⏸ was blocked on rx1"]
        E3["handle ThinkResponse::Do"]
        E2["eval_block (do block body)"]
        E1["eval_think_block (inner)<br/>⏸ blocked on rx2 ← top"]
        E4 --> E3 --> E2 --> E1
    end

    subgraph RedirectStack ["Redirect Stack"]
        direction BT
        R1["T1 (outer)"]
        R2["T2 (inner) ← top"]
        R1 --> R2
    end

    subgraph AsyncTasks ["Async Tasks"]
        direction BT
        A3["redirect_actor<br/>routes to stack top"]
        A2["think_handler T1<br/>⏸ waiting for do result"]
        A1["think_handler T2<br/>⏸ waiting on think_rx2"]
    end
```

The evaluator's call stack and the redirect stack grow in parallel. When the inner think completes, T2 pops off the redirect stack, and the evaluator unwinds back to the outer think's `ThinkResponse::Do` handler.

## Arbitrary Depth

This pattern supports arbitrary nesting depth. Each level:

1. Creates its own response channel pair
2. Pushes a new think handler onto the redirect stack
3. The innermost handler receives all notifications
4. On completion, pops and returns control to the next level

The only limits are:
- Stack space in the evaluator thread (for deeply nested Rust calls)
- Memory for the channel buffers and think handlers

## Implementation Notes

The redirect actor is simple—it just routes to the top of the stack:

```rust
async fn redirect_actor(mut rx: UnboundedReceiver<RedirectMessage>) {
    let mut stack: Vec<Sender<PerSessionMessage>> = vec![];

    while let Some(message) = rx.recv().await {
        match message {
            RedirectMessage::IncomingMessage(msg) => {
                if let Some(sender) = stack.last() {
                    sender.send(msg).await?;
                }
            }
            RedirectMessage::PushThinker(sender) => {
                stack.push(sender);
            }
            RedirectMessage::PopThinker => {
                stack.pop();
            }
        }
    }
}
```

The complexity is in understanding the overall flow, not in any single component.
