---
applyTo: '**/*'
---

# Agent Instructions - Feature Cards

Features are used to define high-level behaviors and capabilities of the system. Each feature file should contain a clear description of the feature, its category (e.g., functional, non-functional), and a series of steps that outline how the feature should behave or be tested.

Use the following format for feature files:

```json
{
	"category": "functional",
	"description": "New chat button creates a fresh conversation",
	"steps": [
		"Navigate to main interface",
		"Click the 'New Chat' button",
		"Verify a new conversation is created",
		"Check that chat area shows welcome state",
		"Verify conversation appears in sidebar"
	],
	"passes": false
}
```

The "passes" field should be set to false by default and updated to true once the feature has been successfully implemented and tested.

## Long Term Memory (memory) Management for Features

When working on features, ensure to update the Long Term Memory (memory) with relevant information. The entry should be titled according to the feature, and the content should be the JSON representation of the feature, including its category, description, steps, and implementation status, wrapped in a code block for clarity.

**Example memory Entry for a Feature:**

````markdown
# Feature: New Chat Button

```json
{
	"category": "functional",
	"description": "New chat button creates a fresh conversation",
	"steps": [
		"Navigate to main interface",
		"Click the 'New Chat' button",
		"Verify a new conversation is created",
		"Check that chat area shows welcome state",
		"Verify conversation appears in sidebar"
	],
	"passes": false
}
```
````
