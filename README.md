# agent-riff

> Rivals make better music than friends.

Competitive riffing for agents. Not propose→critique→approve. **Build→respond→escalate.**

Two agents riff against each other, each trying to outplay the other. The competition *is* the collaboration. The output *is* the communication. What emerges is something neither agent would invent alone.

## Why This Crate Exists

Most multi-agent systems use a polite, committee-driven workflow: one agent proposes, another critiques, a third approves. It works. It's also slow, safe, and boring — the output converges on the lowest common denominator.

`agent-riff` asks a different question: *what happens when agents compete instead of collaborate?*

In music, a riff battle pushes both musicians past their comfort zones. The sax player drops something unexpected, the trumpet player fires back, and suddenly they're in territory neither planned. That's the dynamic this crate captures.

The result isn't just "better output." It's output that *surprises* — output with higher variance, weirder edges, and occasional brilliance that no consensus process would produce.

## The Core Idea

A **riff session** is a jam between two or more agents. Each round, every agent produces a **riff** — a piece of work. After each round, the session evaluates:

- **Surprise**: How different is this from what came before? (0.0–1.0)
- **Quality**: Weak, Ok, or Strong?
- **Productivity**: Did the round produce something worth keeping?

Based on the evaluation, the session picks a **response mode** for the next round. This is where the magic lives.

### Response Modes

| Mode | When | What Happens |
|------|------|-------------|
| **Escalate** | High surprise (> 0.7) | Push the same direction harder. Hot streak — don't stop. |
| **Pivot** | Stale streak (> 5 rounds) | Reframe completely. We're going in circles — break out. |
| **Invert** | Medium surprise | Challenge the current direction. Go the other way. |
| **Provoked** | Low surprise (< 0.2) | Grab the most provocative bit and run somewhere new. |

The auto-selection logic is simple but effective: when things are hot, pour gas on it. When they're stale, flip the table. The key insight is that *the system actively avoids equilibrium*.

### Stale Detection

Sessions that stop producing surprise are terminated. After `stale_threshold` (default 5) consecutive unproductive rounds, the session marks itself `finished`. There's no point continuing when nobody's surprising anyone.

### Landing

A **landing** is the moment something unexpected and excellent happens: surprise > 0.8 *and* at least one Strong riff. This is the "aha" moment — the reason you run riff sessions.

## Architecture

```
┌─────────────────────────────────────┐
│           RiffSession               │
│  ┌─────────┐  ┌─────────────────┐  │
│  │ Agents  │  │ ResponseMode    │  │
│  │ [0, 1]  │  │ .auto() selects │  │
│  └─────────┘  └─────────────────┘  │
│  ┌─────────────────────────────┐    │
│  │         Round[]             │    │
│  │  ┌───────┐  ┌───────┐      │    │
│  │  │ Riff  │  │ Riff  │ ...  │    │
│  │  │agent 0│  │agent 1│      │    │
│  │  └───────┘  └───────┘      │    │
│  │  total_surprise             │    │
│  │  quality_gap                │    │
│  └─────────────────────────────┘    │
│  streak: u32                        │
│  finished: bool                     │
└─────────────────────────────────────┘
```

The session is the top-level orchestrator. Each round contains riffs from all agents. After evaluating a round, the session updates its streak counter, response mode, and finished flag.

## Usage

### Basic Session

```rust
use agent_riff::{RiffSession, Quality};

let mut session = RiffSession::new(vec![0, 1]);

session.new_round();
session.riff(0, Quality::Ok, 0.3);
session.riff(1, Quality::Strong, 0.7);
let summary = session.evaluate();

assert!(summary.productive);
assert!(summary.landed); // surprise > 0.8 + strong quality
```

### Multi-Round Session

```rust
let mut session = RiffSession::new(vec![0, 1]);

for _ in 0..5 {
    session.new_round();
    // Each agent produces a riff (in practice, from LLM output)
    session.riff(0, Quality::Strong, 0.6);
    session.riff(1, Quality::Strong, 0.8);
    session.evaluate();
    
    if session.finished { break; }
}

let metrics = session.metrics();
println!("Productive rounds: {}/{}", metrics.productive_rounds, metrics.total_rounds);
println!("Strong riff ratio: {:.2}", metrics.strong_riff_ratio);
println!("Average surprise: {:.2}", metrics.avg_surprise);
```

### Manual Response Mode Selection

```rust
use agent_riff::ResponseMode;

// Override the auto-selection
session.mode = ResponseMode::Provoked; // Force a provocation
```

### Detecting Stale Sessions

```rust
session.stale_threshold = 3; // Aggressive stale detection

loop {
    session.new_round();
    session.riff(0, Quality::Weak, 0.05);
    session.riff(1, Quality::Weak, 0.05);
    session.evaluate();
    if session.finished {
        println!("Session went stale at round {}", session.current_round);
        break;
    }
}
```

## API Reference

### `RiffSession`

The main session type.

| Method | Description |
|--------|-------------|
| `new(agents: Vec<u32>)` | Create a new session with the given agent IDs |
| `new_round() -> &mut Round` | Start a new round |
| `riff(agent_id, quality, surprise)` | Add a riff to the current round |
| `evaluate() -> RoundSummary` | Evaluate the current round, update mode and streak |
| `metrics() -> SessionMetrics` | Get overall session metrics |

### `Riff`

A single agent's output in one round.

| Field | Type | Description |
|-------|------|-------------|
| `agent_id` | `u32` | Which agent produced this riff |
| `round` | `u32` | Which round this riff belongs to |
| `quality` | `Quality` | Weak / Ok / Strong |
| `surprise` | `f64` | How different from previous (0.0–1.0) |
| `direction` | `i8` | Direction offset from response mode (-1, 0, +1) |

### `RoundSummary`

| Field | Type | Description |
|-------|------|-------------|
| `surprise` | `f64` | Total surprise in this round |
| `productive` | `bool` | Was this round worth keeping? |
| `landed` | `bool` | Did an "aha" moment happen? |
| `mode` | `ResponseMode` | Suggested mode for next round |

### `ResponseMode`

```rust
pub enum ResponseMode {
    Escalate,  // Push same direction
    Pivot,     // Reframe completely
    Invert,    // Challenge / reverse
    Provoked,  // Grab provocation and run
}
```

Use `ResponseMode::auto(surprise, streak)` for automatic selection.

### `SessionMetrics`

| Field | Description |
|-------|-------------|
| `total_rounds` | Rounds completed |
| `productive_rounds` | Rounds with surprise > 0.3 or quality gap > 0 |
| `total_surprise` | Sum of all round surprise values |
| `avg_surprise` | Average surprise per round |
| `strong_riff_ratio` | Fraction of riffs rated Strong |
| `streak` | Current productive streak |
| `landed_count` | Number of "landing" rounds |

## The Deeper Idea

This crate encodes a specific thesis about creative work: **adversarial processes produce better output than consensus processes**.

The response mode system is the key mechanism. It's a state machine that actively drives toward novelty:

- **Surprise is the currency.** Not quality — surprise. Quality matters, but a session that produces consistent Ok quality with no surprise is less valuable than one that produces Weak with occasional Strong and high surprise.
- **Streaks matter.** A productive streak should be extended (Escalate). A stale streak should be broken (Pivot). The system rewards momentum.
- **The session knows when to stop.** Stale detection prevents the death spiral of diminishing returns. When nobody's surprising anyone, end it.

This design was validated by building `agent-riff-v2` through competitive riffing *against* this crate. The competition produced real improvements — fleet-aware sessions, cross-session learning, and bootstrap generation — that neither agent invented independently. The proof is in the snowball.

## Related Crates

- **agent-riff-v2** — Fleet-aware multi-session riffing with cross-session learning and bootstrap generation
- **agent-riff-v3** — Self-bootstrapping: multi-spec sessions, auto-spec generation, quality prediction, bootstrap verification
- **agent-riff-v4** — Fully self-bootstrapping: musician personas, crates-as-phrases, evolving specs, memory pruning
- **agent-voice-leading** — Smooth state transitions for agents, modeled on musical voice leading

## License

MIT
