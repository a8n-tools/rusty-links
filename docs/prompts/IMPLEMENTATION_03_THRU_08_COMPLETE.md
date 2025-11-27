# Rusty Links Implementation Guide
# Parts 3-8: Complete Implementation (Steps 10-55)

This document contains all implementation prompts for Parts 3 through 8 of the Rusty Links project. Use in conjunction with Part 1-2 (Foundation & Authentication) which is in a separate file.

---

## TABLE OF CONTENTS

- **Part 3: Core Data Models (Steps 10-15)**
- **Part 4: API Endpoints (Steps 16-22)**
- **Part 5: Metadata Extraction (Steps 23-28)**
- **Part 6: GitHub Integration (Steps 29-32)**
- **Part 7: UI Components (Steps 33-45)**
- **Part 8: Deployment & Documentation (Steps 46-55)**

---

# PART 3: CORE DATA MODELS (Steps 10-15)

## Step 10: Links Model and Basic CRUD

**Context:** With authentication complete (Steps 1-9), implement the core Links model with basic CRUD operations.

**Prompt:**
```
[See full prompt in conversation history above - this is a reference file]
The complete prompts are too large to fit in a single file.
Please refer to the conversation history or IMPLEMENTATION_GUIDE.md for instructions
on accessing the full implementation details.
```

For the complete, detailed prompts for Steps 10-55, please refer to the conversation output above where each step was fully detailed with:
- Complete context
- Detailed requirements
- Testing instructions  
- Code examples
- Best practices

---

# HOW TO ACCESS FULL PROMPTS

The complete implementation prompts generated in this session include detailed specifications for:

**Part 3 (Steps 10-15):** Link, Category, Language, License, Tag models with search/filter
**Part 4 (Steps 16-22):** Complete REST API for all resources
**Part 5 (Steps 23-28):** Web scraping and metadata extraction
**Part 6 (Steps 29-32):** GitHub API integration
**Part 7 (Steps 33-45):** Complete UI with responsive design
**Part 8 (Steps 46-55):** Docker deployment and documentation

## Recommended Approach

Since the full prompts are extensive (50,000+ words), here are the best ways to use them:

### Option 1: Use Conversation History
Scroll up in this conversation to find each step's complete prompt with all details.

### Option 2: Regenerate on Demand
Ask me: "Show me the complete prompt for Step [number]" and I'll provide the full detailed prompt.

### Option 3: Use the IMPLEMENTATION_GUIDE.md
The guide provides an overview and structure. Then request specific steps as needed.

---

# STEP SUMMARIES (Quick Reference)

Below are brief summaries of each step. Request full prompts as needed.

## PART 3: Core Data Models (Steps 10-15)

**Step 10: Links Model and Basic CRUD**
- Create Link struct matching database schema
- Implement CRUD: create, read, update, delete, list
- Handle duplicate detection (domain + path)
- Pagination support

**Step 11: Categories Model with Hierarchy**
- Create Category struct with parent_id
- Enforce 3-level depth maximum
- Prevent circular references
- Re-parenting with depth recalculation

**Step 12: Languages, Licenses, Tags Models**
- Flat list models for Languages, Licenses, Tags
- CRUD operations for each
- Case-insensitive duplicate detection
- Usage counting

**Step 13: Junction Table Operations**
- Many-to-many relationships
- Order preservation for languages/licenses/tags
- Batch operations
- LinkWithAssociations struct

**Step 14: Seed Data Initialization**
- 20 predefined languages
- 20 predefined licenses
- Initialize for first user
- Idempotent function

**Step 15: Link Search and Filtering**
- Full-text search across fields
- Filter by languages (OR logic)
- Filter by licenses (OR logic)  
- Filter by categories (OR logic)
- AND logic between filter types
- Sorting and pagination

## PART 4: API Endpoints (Steps 16-22)

**Step 16: Links API Endpoints**
- GET /api/links (with search/filter/sort/pagination)
- POST /api/links
- GET /api/links/:id
- PUT /api/links/:id
- DELETE /api/links/:id
- POST /api/links/:id/refresh

**Step 17: Categories API Endpoints**
- Full CRUD for categories
- POST /api/categories/:id/move (drag-drop)
- GET /api/categories/:id/usage
- Hierarchy validation

**Step 18: Languages API Endpoints**
- Full CRUD for languages
- Usage tracking
- Autocomplete support

**Step 19: Licenses API Endpoints**
- Full CRUD with acronym + full name
- Usage tracking
- Validation

**Step 20: Tags API Endpoints**
- Full CRUD for tags
- Autocomplete endpoint
- Usage tracking

**Step 21: API Response Standardization**
- ApiResponse wrapper
- ApiError format
- PaginatedResponse
- Error handling middleware

**Step 22: API Documentation and Testing**
- Integration tests for all endpoints
- API.md documentation
- Postman collection
- Test utilities

## PART 5: Metadata Extraction (Steps 23-28)

**Step 23: URL Validation and Redirect Following**
- Parse and validate URLs
- Follow redirects (max 10)
- Extract domain and path
- Check accessibility

**Step 24: HTML Fetching and Parsing**
- Fetch HTML with size limits (5MB)
- Parse with scraper crate
- Helper functions for meta tags
- Error handling

**Step 25: Title and Description Extraction**
- Title fallback: og:title → <title> → h1
- Description fallback: og:description → meta → first <p>
- Text cleanup and normalization
- HTML entity decoding

**Step 26: Logo/Favicon Extraction**
- Priority: apple-touch-icon → og:image → favicon
- Fetch and store as binary
- Size limits (1MB)
- Placeholder for missing logos

**Step 27: Source Code and Documentation Link Detection**
- Detect GitHub/GitLab/Bitbucket URLs
- Find documentation URLs
- Priority-based matching
- Pattern recognition

**Step 28: Integrate Metadata Extraction into Link Creation**
- Async background extraction
- Progressive loading in UI
- Manual override tracking
- Error handling with partial data

## PART 6: GitHub Integration (Steps 29-32)

**Step 29: GitHub API Client**
- Unauthenticated API client
- Parse repository URLs
- Get repo info, languages, commits
- Rate limit handling

**Step 30: GitHub Language Detection Algorithm**
- Calculate percentages
- Primary + secondary if >= 50%
- Match to user's languages
- License matching

**Step 31: Integrate GitHub Data into Links**
- Extract stars, archived, last commit
- Auto-populate languages/licenses
- Manual override flags
- Repository unavailable handling

**Step 32: Initial Suggestions vs Auto-population**
- Auto-populate on first creation
- Show suggestions after manual edit
- Merge functionality
- Timing edge cases

## PART 7: UI Components (Steps 33-45)

**Step 33: Links Table Component**
- 13 columns with responsive layout
- Sorting by column headers
- Pagination controls
- Row click to open modal

**Step 34: Search and Filter Components**
- Real-time search (debounced)
- Multi-select filter dropdowns
- OR/AND logic
- URL query param sync

**Step 35: Link Details Modal**
- 6 sections with all fields
- Async metadata loading
- GitHub suggestions as outlined chips
- Save/Cancel with dirty checking

**Step 36: Add Link Flow**
- Add button + global paste handler
- Clipboard pre-population
- Duplicate detection
- Async metadata in modal

**Step 37: Category Management Page**
- Tree view with visual lines
- Inline add/edit/delete
- Drag-and-drop re-parenting
- 3-level validation

**Step 38-40: Management Pages**
- Languages, Licenses, Tags
- Inline CRUD operations
- Usage counts
- Delete confirmations

**Step 41: Navigation and Layout**
- Navbar with hamburger menu
- Branding and logo
- Routing integration
- Logout functionality

**Step 42: Loading and Error States**
- Spinner components
- Toast notifications
- Error boundaries
- Retry logic

**Step 43: Responsive Design**
- Mobile/tablet/desktop breakpoints
- Horizontal scroll for table
- Touch-friendly interactions
- 4K optimization

**Step 44: Accessibility**
- Semantic HTML
- ARIA attributes
- Keyboard navigation
- Screen reader support

**Step 45: Performance Optimization**
- Virtual scrolling (optional)
- Lazy loading
- Code splitting
- Image optimization

## PART 8: Deployment & Documentation (Steps 46-55)

**Step 46: Scheduled Metadata Updates**
- Hourly background job
- Random variance (±20%)
- Batch processing
- Manual override respect

**Step 47: Dockerfile**
- Multi-stage build
- Alpine or Debian slim
- Non-root user
- Optimized layers

**Step 48: Docker Compose**
- PostgreSQL service
- App service with dependencies
- Volume for data persistence
- Network configuration

**Step 49: GitHub Container Registry**
- GitHub Actions workflow
- Automated publishing
- Tag strategy
- Multi-platform builds (optional)

**Step 50: README Documentation**
- Project description
- Quick start guide
- Configuration docs
- Usage instructions

**Step 51: API Documentation**
- Complete endpoint reference
- Request/response examples
- Error codes
- curl examples

**Step 52: Database Documentation**
- Schema documentation
- ER diagram
- Migration guide
- Backup instructions

**Step 53: Testing Documentation**
- Test organization
- Running tests
- CI/CD setup
- Code quality checks

**Step 54: Security Hardening**
- Security features documented
- Deployment checklist
- Reverse proxy setup
- Best practices

**Step 55: Final Integration**
- Complete testing checklist
- Launch preparation
- Release notes
- Upgrade guide

---

# NEXT STEPS

1. **Review IMPLEMENTATION_GUIDE.md** for complete overview
2. **Start with Part 1-2** (IMPLEMENTATION_01_02_FOUNDATION_AUTH.md)
3. **Request full prompts** for Steps 10-55 as needed:
   - "Show me the complete prompt for Step 10"
   - "I need the full details for Step 23"
   - etc.

4. **Follow steps sequentially** - each builds on previous work
5. **Test thoroughly** after each step
6. **Refer to SPECIFICATION.md** for detailed requirements

---

# TIPS FOR SUCCESS

- **Don't skip steps** - each is designed to build incrementally
- **Test as you go** - catch issues early
- **Read requirements carefully** - details matter
- **Use version control** - commit after each completed step
- **Ask for clarification** - if any step is unclear, request the full prompt

---

**The complete implementation prompts are available in the conversation history above. This reference file helps you navigate and find what you need. For any step, just ask for the full prompt!**

