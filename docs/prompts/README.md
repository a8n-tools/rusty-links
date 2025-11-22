# Rusty Links Implementation Prompts

This directory contains step-by-step prompts for implementing the Rusty Links application using a code-generation LLM.

## Overview

The project is divided into 9 parts with 45 total steps. Each prompt is designed to be:
- **Incremental**: Builds on previous steps
- **Self-contained**: Can be executed independently
- **Verifiable**: Includes verification steps
- **Integrated**: No orphaned code

## Parts Summary

| Part | Steps | Description                                   | Status      |
|------|-------|-----------------------------------------------|-------------|
| 1    | 1-3   | Foundation (scaffolding, config, errors)      | ✅ Complete  |
| 2    | 4-9   | Authentication (DB, users, sessions, API, UI) | ✅ Complete  |
| 3    | 10-15 | Link Management Core                          | 📋 Ready    |
| 4    | 16-21 | Categorization System                         | 📋 Ready    |
| 5    | 22-26 | Metadata (Languages/Licenses)                 | 📋 Ready    |
| 6    | 27-31 | External Integrations (Scraper, GitHub)       | 📋 Ready    |
| 7    | 32-35 | Background Processing                         | 📋 Ready    |
| 8    | 36-40 | Search & Filtering                            | 📋 Ready    |
| 9    | 41-45 | Polish & Advanced Features                    | 📋 Ready    |

## Files

- [BUILD_BLUEPRINT.md](../BUILD_BLUEPRINT.md) - High-level project blueprint
- [PART3_LINK_MANAGEMENT.md](./PART3_LINK_MANAGEMENT.md) - Steps 10-15
- [PART4_CATEGORIZATION.md](./PART4_CATEGORIZATION.md) - Steps 16-21
- [PART5_METADATA.md](./PART5_METADATA.md) - Steps 22-26
- [PART6_INTEGRATIONS.md](./PART6_INTEGRATIONS.md) - Steps 27-31
- [PART7_BACKGROUND.md](./PART7_BACKGROUND.md) - Steps 32-35
- [PART8_SEARCH_FILTER.md](./PART8_SEARCH_FILTER.md) - Steps 36-40
- [PART9_POLISH.md](./PART9_POLISH.md) - Steps 41-45

## How to Use

1. **Start with Part 3, Step 10** (Parts 1-2 are already implemented)
2. Copy the prompt text from the relevant step
3. Provide it to your code-generation LLM
4. Verify the output compiles: `cargo check`
5. Test functionality manually if needed
6. Proceed to the next step

## Resume Point

**Current position: Part 3, Step 10 (Link Model)**

The authentication system is complete. Next steps:
1. Create the Link model
2. Implement Link CRUD API
3. Build the Links UI page

## Dependencies to Add

As you progress, you may need to add dependencies:

```toml
# Part 3 - URL parsing
url = "2.5"

# Part 6 - Web scraping
scraper = "0.18"

# Part 8 - URL encoding for search
urlencoding = "2.1"
```

## Testing

After each step:
1. Run `cargo check` - must pass
2. Run `cargo build --release` - should succeed
3. Test the feature manually using the UI or curl

## Notes

- Each prompt includes context about what's already implemented
- Prompts use existing patterns from the codebase
- Error handling consistently uses `AppError`
- All API endpoints require authentication (except auth endpoints)
