# Perception and Knowledge

Partial observation is central to both gameplay and AI-agent design.

## Layers

### Truth

The authoritative world state. Only the engine and debug tools can see this
directly.

### Perception

What an actor currently senses.

Perception may include:

- sight
- sound
- smell
- touch
- magical detection
- social recognition
- remembered map knowledge

### Memory

What an actor previously perceived.

Memory should be allowed to become stale. An actor may remember that a guard was
at a door ten turns ago, even if the guard has moved.

### Belief

What an actor thinks is true.

Belief can come from memory, rumor, culture, fear, lies, books, faction
knowledge, or inference. Belief can be false.

### Knowledge

Stable or verified information available to the actor.

Examples:

- known recipes
- known faction symbols
- known names
- known map locations
- known rituals
- known laws

## Agent Consequence

An AI agent should receive an actor-specific observation package, not the whole
world.

This makes different actors naturally act differently:

- a blind actor may rely on sound
- a scholar may recognize a relic
- a guard may know local law
- a thief may know secret passages
- a beast may smell blood but not understand writing

