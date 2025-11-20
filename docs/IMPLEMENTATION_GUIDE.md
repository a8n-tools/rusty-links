# Rusty Links - Complete Implementation Guide

## Overview

This guide provides step-by-step implementation prompts for building Rusty Links, a self-hosted bookmark manager built with Rust and Dioxus. The guide is divided into 8 parts covering 55 incremental steps.

## Purpose

These prompts are designed to be given to a code-generation LLM to implement each feature incrementally. Each step builds on previous steps, ensuring no orphaned code and proper integration at every stage.

## Implementation Philosophy

- **Small, incremental steps** - Each step is safely implementable without big jumps in complexity
- **Build and integrate** - Every step ends with working, integrated code
- **Test as you go** - Each step includes testing requirements
- **No hanging code** - All code is wired together, nothing is orphaned

## Implementation Parts

### Part 1-2: Foundation & Authentication (Steps 1-9)
**File:** `IMPLEMENTATION_01_02_FOUNDATION_AUTH.md`

Covers the foundational setup and authentication system:
- Step 1: Project Initialization
- Step 2: Configuration Management
- Step 3: Database Schema and Initial Migration
- Step 4: Database Connection Pool
- Step 5: Error Handling Framework
- Step 6: User Model and Database Operations
- Step 7: Session Management
- Step 8: Authentication API Endpoints
- Step 9: Authentication UI Components

**Completion:** Foundation established with working authentication

---

### Part 3: Core Data Models (Steps 10-15)
**File:** `IMPLEMENTATION_03_MODELS.md`

Implements all core data models and their operations:
- Step 10: Links Model and Basic CRUD
- Step 11: Categories Model with Hierarchy
- Step 12: Languages, Licenses, and Tags Models
- Step 13: Junction Table Operations
- Step 14: Seed Data Initialization
- Step 15: Link Search and Filtering

**Completion:** All data models working with full CRUD and search

---

### Part 4: API Endpoints (Steps 16-22)
**File:** `IMPLEMENTATION_04_API.md`

Creates RESTful API for all resources:
- Step 16: Links API Endpoints
- Step 17: Categories API Endpoints
- Step 18: Languages API Endpoints
- Step 19: Licenses API Endpoints
- Step 20: Tags API Endpoints
- Step 21: API Response Standardization
- Step 22: API Documentation and Testing

**Completion:** Complete REST API with tests and documentation

---

### Part 5: Metadata Extraction (Steps 23-28)
**File:** `IMPLEMENTATION_05_METADATA.md`

Implements web scraping and metadata extraction:
- Step 23: URL Validation and Redirect Following
- Step 24: HTML Fetching and Parsing
- Step 25: Title and Description Extraction
- Step 26: Logo/Favicon Extraction
- Step 27: Source Code and Documentation Link Detection
- Step 28: Integrate Metadata Extraction into Link Creation

**Completion:** Automatic metadata extraction from any URL

---

### Part 6: GitHub Integration (Steps 29-32)
**File:** `IMPLEMENTATION_06_GITHUB.md`

Adds GitHub repository integration:
- Step 29: GitHub API Client
- Step 30: GitHub Language Detection Algorithm
- Step 31: Integrate GitHub Data into Links
- Step 32: Initial Suggestions vs. Auto-population

**Completion:** GitHub integration with stars, languages, licenses

---

### Part 7: UI Components (Steps 33-45)
**File:** `IMPLEMENTATION_07_UI.md`

Builds the complete user interface:
- Step 33: Links Table Component
- Step 34: Search and Filter Components
- Step 35: Link Details Modal
- Step 36: Add Link Flow
- Step 37: Category Management Page
- Step 38: Languages Management Page
- Step 39: Licenses Management Page
- Step 40: Tags Management Page
- Step 41: Navigation and Layout
- Step 42: Loading and Error States
- Step 43: Responsive Design and Mobile Optimization
- Step 44: Accessibility Improvements
- Step 45: Performance Optimization

**Completion:** Full-featured responsive web interface

---

### Part 8: Deployment & Documentation (Steps 46-55)
**File:** `IMPLEMENTATION_08_DEPLOYMENT.md`

Handles deployment, scheduling, and documentation:
- Step 46: Scheduled Metadata Updates - Background Job
- Step 47: Dockerfile - Multi-stage Build
- Step 48: Docker Compose Configuration
- Step 49: GitHub Container Registry Publishing
- Step 50: Comprehensive README Documentation
- Step 51: API Documentation
- Step 52: Database Migrations Documentation
- Step 53: Testing Documentation and Test Suite Completion
- Step 54: Security Hardening Checklist
- Step 55: Final Integration and Launch Preparation

**Completion:** Production-ready deployment with complete documentation

---

## How to Use This Guide

### For Developers

1. **Sequential Implementation**
   - Follow steps in order (1-55)
   - Complete each step fully before moving to the next
   - Test after each step

2. **Using with LLMs**
   - Copy the entire prompt for each step
   - Paste into your code-generation LLM (Claude, GPT-4, etc.)
   - Review the generated code
   - Test and verify before proceeding

3. **Adaptation**
   - Prompts can be adapted for your specific needs
   - Technology stack can be swapped (e.g., different framework)
   - Steps can be split further if needed

### For Code-Generation LLMs

Each prompt is structured as:
- **Context:** What's been built so far
- **Requirements:** Detailed specifications for this step
- **Test:** How to verify the step works

Follow the requirements exactly, then integrate the code with previous steps.

---

## Project Structure

After completing all steps, your project will have:

```
rusty-links/
├── src/
│   ├── main.rs              # Application entry point
│   ├── config.rs            # Configuration management
│   ├── error.rs             # Error handling
│   ├── models/              # Database models
│   │   ├── user.rs
│   │   ├── link.rs
│   │   ├── category.rs
│   │   ├── language.rs
│   │   ├── license.rs
│   │   ├── tag.rs
│   │   ├── link_associations.rs
│   │   ├── link_search.rs
│   │   └── seed.rs
│   ├── api/                 # REST API endpoints
│   │   ├── auth.rs
│   │   ├── links.rs
│   │   ├── categories.rs
│   │   ├── languages.rs
│   │   ├── licenses.rs
│   │   ├── tags.rs
│   │   └── response.rs
│   ├── auth/                # Authentication
│   │   └── session.rs
│   ├── scraper/             # Metadata extraction
│   │   ├── url_utils.rs
│   │   ├── html_fetcher.rs
│   │   ├── metadata.rs
│   │   ├── logo.rs
│   │   ├── links.rs
│   │   └── extractor.rs
│   ├── github/              # GitHub integration
│   │   ├── client.rs
│   │   └── languages.rs
│   ├── scheduler/           # Background jobs
│   │   └── update_job.rs
│   └── ui/                  # Frontend components
│       ├── app.rs
│       ├── pages/
│       │   ├── setup.rs
│       │   ├── login.rs
│       │   ├── links_list.rs
│       │   ├── categories.rs
│       │   ├── languages.rs
│       │   ├── licenses.rs
│       │   └── tags.rs
│       └── components/
│           ├── navbar.rs
│           ├── layout.rs
│           ├── search_filter.rs
│           ├── link_modal.rs
│           ├── add_link_button.rs
│           ├── loading.rs
│           └── error.rs
├── migrations/              # Database migrations
│   ├── 001_initial_schema.sql
│   ├── 002_seed_data.sql
│   └── 003_sessions.sql
├── tests/                   # Integration tests
│   └── api/
├── Dockerfile
├── compose.yml
├── .env.example
└── README.md
```

---

## Technology Stack

### Backend
- **Language:** Rust (latest stable)
- **Framework:** Dioxus Fullstack
- **Database:** PostgreSQL with SQLx
- **HTTP Server:** Axum
- **Password Hashing:** Argon2
- **Web Scraping:** reqwest + scraper
- **Logging:** tracing

### Frontend
- **Framework:** Dioxus Web
- **Styling:** CSS with rust color theme
- **Responsive:** Mobile-first design

### Deployment
- **Container:** Docker multi-stage build
- **Orchestration:** Docker Compose
- **Registry:** GitHub Container Registry

---

## Key Features Implemented

✅ Single-user authentication with Argon2
✅ Link management with CRUD operations
✅ Automatic metadata extraction (title, description, logo)
✅ GitHub repository integration (stars, languages, licenses)
✅ Hierarchical categories (3-level max)
✅ Languages, licenses, and tags management
✅ Full-text search and filtering
✅ Scheduled background updates
✅ Responsive web interface
✅ Docker deployment
✅ Complete REST API
✅ Comprehensive documentation

---

## Phase 2 Features (Future)

The following features are planned for Phase 2:

- Multi-user support with roles
- Public/private links
- Dark mode
- Browser extension
- Data import/export
- Advanced search operators
- OAuth authentication
- Rate limiting and security hardening
- Mobile app
- And more...

---

## Acceptance Criteria

Phase 1 is complete when all 23 acceptance criteria from the specification are met:

1. ✅ User can create account on first run
2. ✅ User can log in with email/password
3. ✅ User can add links via button or paste
4. ✅ System extracts metadata automatically
5. ✅ System detects and prevents duplicates
6. ✅ GitHub integration fetches repository data
7. ✅ User can view links in sortable, paginated table
8. ✅ User can search across all fields
9. ✅ User can filter by language, license, category
10. ✅ User can edit links with all associations
11. ✅ User can delete links
12. ✅ User can manage categories with drag-drop
13. ✅ User can manage languages, licenses, tags
14. ✅ Scheduled job updates metadata automatically
15. ✅ Application deployed via Docker Compose
16. ✅ Comprehensive tests pass
17. ✅ README documentation complete
18. ✅ All error handling implemented
19. ✅ Responsive design works on all devices
20. ✅ Session management works correctly
21. ✅ Logout functionality works
22. ✅ All validation rules enforced
23. ✅ Logging configured and working

---

## Getting Started

1. Read the specification: `SPECIFICATION.md`
2. Start with Part 1-2: Foundation & Authentication
3. Follow steps sequentially
4. Test thoroughly at each step
5. Review generated code before proceeding
6. Refer back to specification for clarifications

---

## Support and Feedback

This implementation guide is designed to be comprehensive and self-contained. Each step builds incrementally on the previous work, ensuring a solid foundation at every stage.

For questions or issues with the specification, refer to `SPECIFICATION.md`.

---

**Ready to build Rusty Links? Start with Step 1!**
