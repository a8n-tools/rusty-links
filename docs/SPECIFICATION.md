# Rusty Links - Phase 1 Product Specification

## Project Overview

**Project Name:** Rusty Links  
**Repository:** `rusty-links`  
**Description:** A self-hosted bookmark manager built with Rust and Dioxus, designed to help users organize, categorize, and track links with automatic metadata extraction and GitHub integration.

**License:** MIT

---

## Phase 1 Scope

Phase 1 focuses on a single-user, self-hosted deployment with core bookmark management functionality:

- Email/password authentication (single user)
- Add, view, edit, delete links
- Automatic metadata extraction from web pages
- GitHub repository integration
- Categorization with tags, purposes, languages, and licenses
- Search and filtering
- Scheduled metadata updates
- Docker deployment with PostgreSQL

---

## Technology Stack

### Backend
- **Language:** Rust (latest stable version)
- **Framework:** Dioxus Fullstack (single binary for server + frontend)
- **Database:** PostgreSQL
- **ORM:** SQLx with compile-time checked queries and migrations
- **Password Hashing:** Argon2
- **HTTP Client:** reqwest
- **HTML Parsing:** scraper
- **Logging:** Structured JSON logging (use standard Rust logging library with configurable levels)

### Frontend
- **Framework:** Dioxus Web (served by Dioxus Fullstack)
- **Styling:** Responsive design with professional/muted rust color tones
- **API:** RESTful API endpoints under `/api`

### Deployment
- **Containerization:** Docker with multi-stage build
- **Base Image:** Alpine or Distroless
- **Orchestration:** Docker Compose
- **Registry:** GitHub Container Registry

---

## Architecture

### Project Structure
- Monorepo with clear separation between frontend and backend code
- Modular organization:
    - Database models
    - API/server functions
    - UI components
    - Business logic
    - Scraping/metadata extraction
    - GitHub API client
    - Authentication
    - Configuration management

### API Design
- RESTful conventions (GET, POST, PUT, DELETE)
- Base path: `/api`
- Standard JSON responses with metadata
- Error responses follow Rust RESTful community guidelines with format:
```json
{
  "error": "Error message",
  "code": "ERROR_CODE",
  "status": 400
}
```

---

## Authentication & User Management

### Initial Setup
- On first run, check if database exists and user exists
- If database doesn't exist, run database migrations automatically on startup
- If user doesn't exist, display setup page for account creation
- After account creation, redirect to login page
- Setup page becomes inaccessible after first user is created

### Account Creation
- Email field (basic validation: contains @, has domain)
- Password field (no requirements in Phase 1)
- No password strength requirements or complexity rules
- Password stored with Argon2 hash using recommended parameters

### Session Management
- Secure, httpOnly cookies for session tokens
- SameSite cookie attributes for CSRF protection
- Authentication tokens passed in headers for API requests
- Sessions persist indefinitely until explicit logout
- Support multiple concurrent sessions from different devices/browsers
- Session expires: redirect to login page on next action
- No rate limiting on login attempts in Phase 1

### Login
- Email/password authentication
- Invalid credentials: generic "Invalid credentials" message (don't reveal if user exists)
- No "remember me" option

### Logout
- Logout button in menu
- No confirmation required
- Logs out current session only
- Redirects to login page

---

## Data Model

### Users Table
- `id` (primary key)
- `email` (unique, validated format)
- `password_hash` (Argon2)
- `created_at` (timestamp)

### Links Table
- `id` (primary key)
- `user_id` (foreign key)
- `url` (text) - final URL after following redirects
- `domain` (text) - extracted for duplicate detection
- `path` (text) - extracted for duplicate detection
- `title` (text, nullable) - auto-extracted page title
- `description` (text, nullable) - auto-extracted description
- `logo` (bytea, nullable) - stored as binary data
- `source_code_url` (text, nullable) - user-editable
- `documentation_url` (text, nullable) - user-editable
- `notes` (text, nullable) - user markdown notes
- `status` (enum: 'active', 'archived', 'inaccessible', 'repo_unavailable')
- `github_stars` (integer, nullable)
- `github_archived` (boolean, nullable)
- `github_last_commit` (date, nullable)
- `created_at` (timestamp)
- `updated_at` (timestamp) - last manual edit by user
- `refreshed_at` (timestamp, nullable) - last GitHub metadata refresh
- Unique constraint on (domain, path) per user

### Categories Table (Hierarchical)
- `id` (primary key)
- `user_id` (foreign key)
- `name` (text) - globally unique per user (case-insensitive)
- `parent_id` (foreign key, nullable, self-referential)
- `depth` (integer) - for enforcing 3-level maximum
- `sort_order` (integer, nullable) - for sibling ordering
- `created_at` (timestamp)

### Languages Table
- `id` (primary key)
- `user_id` (foreign key)
- `name` (text) - unique per user (case-insensitive)
- `created_at` (timestamp)

Initial seed data (20 languages):
1. JavaScript, 2. Python, 3. Java, 4. C#, 5. C++, 6. TypeScript, 7. PHP, 8. C, 9. Ruby, 10. Go, 11. Rust, 12. Swift, 13. Kotlin, 14. R, 15. Dart, 16. Scala, 17. Perl, 18. Lua, 19. Haskell, 20. Elixir

### Licenses Table
- `id` (primary key)
- `user_id` (foreign key)
- `name` (text) - unique per user (case-insensitive)
- `full_name` (text) - displayed in management screen
- `created_at` (timestamp)

Initial seed data (20 licenses with acronyms):
1. MIT, 2. Apache-2.0, 3. GPL-3.0, 4. BSD-3-Clause, 5. GPL-2.0, 6. BSD-2-Clause, 7. LGPL-3.0, 8. LGPL-2.1, 9. ISC, 10. MPL-2.0, 11. AGPL-3.0, 12. Unlicense, 13. CC0-1.0, 14. EPL-2.0, 15. EUPL-1.2, 16. Ms-PL, 17. WTFPL, 18. Artistic-2.0, 19. Zlib, 20. BSL-1.0

### Tags Table
- `id` (primary key)
- `user_id` (foreign key)
- `name` (text) - unique per user (case-insensitive)
- `created_at` (timestamp)

### Junction Tables (Many-to-Many)
- `link_categories` (link_id, category_id)
- `link_languages` (link_id, language_id, order)
- `link_licenses` (link_id, license_id, order)
- `link_tags` (link_id, tag_id, order)

Note: `order` field preserves user's entry order for display purposes

---

## Core Features

### 1. Link Management

#### Adding Links

**Trigger Methods:**
1. Click "Add Link" button (primary button with icon + text, top of links table)
    - Check clipboard for URL and pre-populate if valid
    - Show empty dialog if clipboard doesn't contain valid URL
2. Global paste handler (Ctrl+V)
    - Only active when NOT focused in an input field
    - Trim whitespace from pasted content
    - If pure URL detected (http/https only), open Add Link dialog with URL pre-populated
    - Ignore paste if Add Link modal already open

**Add Link Flow:**
1. Dialog appears with URL input field
2. User can edit URL or click OK
3. System validates URL:
    - Must be http/https protocol only
    - Must be accessible (attempt connection)
    - If inaccessible, warn user: "The URL couldn't be accessed before saving. It's possible the link is an internal link that is not available from this location."
4. Follow redirects and use final destination URL
5. Check for duplicates using domain + path (ignore query parameters and anchors)
    - If duplicate exists, show existing link details modal (no save)
    - If not duplicate, save to database and show details modal
6. Asynchronously fetch metadata:
    - Display loading spinner for each field being fetched
    - Show progress indicator for pending/completed fields
    - User can start editing other fields while metadata loads
    - Fields appear instantly when data arrives

**URL Storage:**
- Store final URL after following redirects
- Extract and store domain and path separately for duplicate detection
- Display stored URL in all views

#### Metadata Extraction

**Source Priority for Title:**
1. Open Graph title (`og:title`)
2. `<title>` tag
3. Twitter Card title (`twitter:title`)
4. `<h1>` heading
5. Clean up: remove site suffixes, extra whitespace

**Source Priority for Description:**
1. GitHub repository description (if GitHub source link found)
2. Open Graph description (`og:description`)
3. Meta description (`<meta name="description">`)
4. Twitter Card description (`twitter:description`)
5. First paragraph of content

**Logo/Favicon Extraction:**
1. Prefer larger/higher quality: Apple touch icon, Open Graph image
2. Fall back to favicon
3. Store original size as binary data (BYTEA) in database
4. Clean up orphaned logos when links deleted
5. If no logo found: use rusty chain links placeholder (same for all)

**Source Code Link Detection:**
Priority order:
1. Specific meta tags or structured data
2. Links in header/navigation
3. Links in footer
4. Links in main content
5. Any link matching github.com/gitlab.com patterns

Match patterns: github.com, gitlab.com, and other git hosting
- Prioritize links with anchor text: "Source Code", "GitHub", "Repository", "GitLab"
- Use first match found
- User can manually add/override if system fails

**Documentation Link Detection:**
Priority order (use first match):
1. URLs containing: "docs.", "documentation."
2. URLs containing: "/docs/", "/documentation/"
3. Links with anchor text: "Documentation", "Docs", "API Reference", "Guide"
4. Specific documentation platforms: ReadTheDocs, GitBook, Docusaurus
5. Meta tags indicating documentation

**Web Scraping Configuration:**
- HTTP GET requests only (no headless browser)
- Reasonable timeout (follow best practices)
- Parse HTML with scraper library
- No custom user agent
- Ignore robots.txt (linking to main sites, not crawling)
- Look for links in visible page content and page source/comments
- Follow meta refresh redirects
- Check canonical URLs
- Limit scanning to first X KB for performance
- Set maximum page size to fetch
- Timeout if parsing takes too long
- Check all meta tags and structured data

**Error Handling:**
- If webpage fetch fails: save link with status "active", show error in detail modal
- Log all errors to stdout for Docker
- Display user-friendly, specific error messages in UI
- Store partial data if some extractions succeed
- Show error bubble/tooltip on failed fields in details modal
- Next scheduled update will retry failed extractions

#### GitHub Integration

**Detection:**
- Only process links with `github.com/user/repo` format in source code URL

**API Calls (3 per repository):**
1. Repository endpoint: `GET /repos/{owner}/{repo}`
    - Stars count
    - Archived status
    - License
    - Repository description (not used in Phase 1)
2. Languages endpoint: `GET /repos/{owner}/{repo}/languages`
    - Language breakdown by percentage
3. Commits endpoint: `GET /repos/{owner}/{repo}/commits`
    - Last commit date

**Unauthenticated API:**
- Use public GitHub API (60 requests/hour per IP)
- No GitHub Personal Access Token required
- No fallback if rate limited
- Log rate limit errors

**Language Detection Algorithm:**
1. Use the language with the largest percentage
2. Check the language with 2nd largest percentage
3. If 2nd language percentage ≥ 50% of 1st language percentage, include both
4. Do not check 3rd language

Example:
- 75% Rust, 15% JavaScript → Use only Rust
- 50% JavaScript, 40% Elixir → Use both
- 60% Python, 25% JavaScript, 15% HTML → Use only Python (25% < 30%)

**License Detection:**
- Import licenses reported by GitHub
- Only associate if license exists in user's license list
- If GitHub reports license not in list, user must manually add license to system first
- Support multiple licenses (dual-licensed projects)

**Initial Population:**
- Auto-populate languages and licenses only if user hasn't manually selected any
- Show as suggestions in details modal if different from user selections
- Suggestions displayed next to editable fields with different styling (outlined vs. solid)
- Clicking suggestion adds to existing values (merge)
- Suggestions disappear when modal closes

**Data Refresh:**
- `refreshed_at` timestamp updated only when GitHub data changed
- Used to calculate next refresh time
- Initial `refreshed_at` set after first successful metadata fetch
- Formula: `created_at` + `UPDATE_INTERVAL_DAYS` ± 20% random variance (for first refresh)
- Subsequent: `refreshed_at` + `UPDATE_INTERVAL_DAYS` ± 20% random variance
- Variance recalculated after each update

**Update Behavior:**
- Only update database if values actually changed
- Log when values change (e.g., stars increased, archived)
- If repository deleted/inaccessible:
    - Keep last known values
    - Override main link status to "repo_unavailable"
    - Update as searchable/sortable field
- Save partial data if some API calls succeed
- Log specific fields that failed
- Continue processing other links if one fails

**Display in UI:**
- GitHub-specific fields (stars, archived, last commit) only visible when GitHub source URL exists
- Fields greyed out if no GitHub URL
- Format:
    - Stars: formatted with k notation (1.2k, 15.3k)
    - Archived status: badge/label
    - Last commit date: ISO format YYYY-MM-DD (user's locale/timezone)

**Non-GitHub Repositories:**
- GitLab, Bitbucket, self-hosted Git: store URL but leave GitHub fields empty
- Support for other platforms in Phase 2

#### Link Details Modal

**Trigger:**
- Click any row in links table to open modal

**Layout:**
Grouped into sections with clear visual distinction:

**Section 1: Basic Information**
- Logo (display only, different background color)
- Title (display only, different background color)
- URL (editable, standard input)
    - Show as text with "open" icon to open in new tab
    - Validation: cannot be duplicate of another link (domain + path)
    - Show green checkmark if accessible, red X if not
- Description (display only, different background color)

**Section 2: Links**
- Source Code URL (editable, standard input)
    - Show as text with "open" icon
    - Validation status indicator
- Documentation URL (editable, standard input)
    - Show as text with "open" icon
    - Validation status indicator

**Section 3: Categorization**
- Tags (multi-select, autocomplete based on existing tags)
    - Selected shown as chips with X to remove
    - Display in order entered by user
- Purposes/Categories (multi-select from hierarchy)
    - Tree view with 3-level hierarchy, indented display
    - Selected shown as chips (remove from chips only)
    - Highlighted in tree with checkmarks
    - Full paths displayed (e.g., "Software > Open Source > Library")
    - Scrollable if tree is long
- Languages (multi-select)
    - Selected shown as chips with X to remove
    - Display in order entered by user
    - GitHub suggestions shown with outlined styling if different
- Licenses (multi-select)
    - Selected shown as chips with X to remove
    - Display in order entered by user
    - GitHub suggestions shown with outlined styling if different

**Section 4: GitHub Information**
- Only visible when GitHub source URL exists
- Stars (display only, formatted)
- Archived status (badge)
- Last commit date (ISO format)
- "Refresh Metadata" button (icon + text)
    - Asynchronous refresh of all auto-extracted fields
    - Show loading spinner per field
    - Block only fields being updated, not user-editable fields
    - Updates `updated_at` timestamp

**Section 5: Notes**
- Multiline textarea for user notes
- Plain text input, rendered as Markdown on display
- Editable

**Section 6: Metadata**
- Status (display only)
- Created At (display only, ISO date)
- Updated At (display only, ISO date)
- Refreshed At (display only, ISO date, only if exists)

**Modal Behavior:**
- Save and Cancel buttons at bottom
- Save button disabled until user makes a change
- On save: validates, saves all changes, auto-closes modal
- On cancel: closes immediately without warning (implies discard)
- On close (X, Escape, backdrop click):
    - If unsaved changes: show confirmation "Save changes before closing?" with Save/Discard/Cancel buttons
    - If no changes: close immediately
- Maximum modal size with scrolling when needed
- No animation on open/close
- Update URL route to reflect modal state

**Async Loading in Modal:**
- After adding new link, details modal opens immediately
- Metadata fetches asynchronously in background
- Each field shows loading spinner while fetching
- Progress indicator shows overall status
- Fields appear instantly when data arrives
- User can edit fields while loading continues

#### Editing Links

**Workflow:**
- User makes changes in details modal
- Changes tracked but not saved automatically
- Save button enables when changes detected
- Click Save to commit all changes at once
- Validates changes (e.g., URL not duplicate)
- Updates `updated_at` timestamp
- Auto-closes modal on successful save

**Auto-save Removed:**
- No debouncing or auto-save behavior
- Explicit Save/Cancel required

**Validation:**
- URLs must be http/https
- URLs must not be duplicates (domain + path)
- Show inline validation errors
- User-friendly error messages with specifics

#### Deleting Links

**Trigger:**
- Delete button/icon in details modal

**Confirmation:**
- Simple "Are you sure?" dialog
- No additional warnings

**Cleanup:**
- Remove link record
- Remove all junction table entries (categories, languages, licenses, tags)
- Clean up orphaned logo

### 2. Links List (Table View)

#### Display

**Table Columns (left to right):**
1. Logo (icon, responsive size - larger on 4k)
2. Title (truncated with ellipsis)
3. Domain/URL (domain + path, e.g., `example.com/blog`)
4. Description (truncated with ellipsis)
5. Tags (show 2, "..." for 3+)
6. Purposes/Categories (show 2 full paths, "..." for 3+)
7. Languages (show 2, "..." for 3+)
8. Licenses (show 2, "..." for 3+)
9. Status (color-coded badge: active/archived/inaccessible/repo_unavailable)
10. GitHub Stars (formatted: 1.2k)
11. Created At (ISO date)
12. Updated At (ISO date)
13. Refreshed At (ISO date)

**Table Behavior:**
- Full width of viewport (utilize entire screen, especially on 4k monitors)
- Responsive: truncate with ellipsis as screen gets smaller
- Landscape mode for mobile viewing
- Horizontal scroll enabled for smaller screens
- All columns shown (no hiding)

**Row Interaction:**
- Click any row to open details modal
- Click domain/URL to open link in new tab

**Empty State:**
- Show "No links found" message when table is empty
- Same message for no links vs. no filtered results

#### Sorting

**Behavior:**
- Click column header to sort
- Single-column sort only (clicking new column replaces previous)
- Visual arrow indicators on sorted column (up/down for asc/desc)
- Toggle direction on repeated clicks

**Default Sort:**
- Date added (newest first)

**Sort Persistence:**
- No persistence between sessions
- Maintain sort within current session when navigating away and back
- Maintain sort when applying filters

**Multi-value Column Sorting:**
- Tags, purposes, languages, licenses: sort by first value in user-entered order
- Empty values sort to bottom

**Implementation:**
- Use appropriate Rust library that provides sortable table functionality

#### Pagination

**Controls Location:**
- Bottom of table only

**Display Format:**
- Page numbers: 1 2 3 ... 45 46 (with ellipsis for many pages)
- Current page centered: ... 8 9 **10** 11 12 ...
- Always show first and last page
- Previous/Next buttons
- Items per page selector: 20/50/100/200
- Total count: "Showing 1-20 of 150 links"

**Behavior:**
- Default: 20 items per page
- No persistence of page size between sessions
- Maintain current page when returning from details modal

#### Search

**Search Input:**
- Prominent search field at top of table
- Placeholder: "Search links..."
- Real-time search as user types

**Search Scope:**
- Everything: title, description, URL, domain, path, tags, purposes, languages, licenses, notes, status, GitHub fields
- Fuzzy matching (handles typos)
- Case-insensitive

**Search Results:**
- Ranked by relevance (exact matches first, then partial matches)
- Maintain current sort order
- "No links found" if no results

**Advanced Operators:**
- Phase 2 feature (AND, OR, NOT)

#### Filtering

**Filter Dropdowns:**
Three searchable multi-select dropdowns:
1. Languages
2. Licenses
3. Purposes/Categories

**Dropdown Behavior:**
- Searchable: type to filter available options
- Case-insensitive matching anywhere in text
- Show all options initially, filter as user types
- Multi-select with chips/badges showing selections
- X icon on chips to remove individual selections
- Full hierarchy paths shown for categories
- Scrollable if many options

**Filter Logic:**
- Within same dropdown: OR logic (Python OR Rust)
- Between dropdowns: AND logic (Python AND MIT)
- Dropdowns AND with search field

**Implementation:**
- Hybrid approach: client-side for small lists, server-side for large lists

**Reset Filters:**
- Single "Reset search" button to clear all search and filters

**Filter Persistence:**
- No persistence between sessions
- Filters cleared when navigating away

### 3. Category Management

**Navigation:**
- "Categories" menu item

**Page Layout:**
- Tree view with visual connecting lines (no folder icons)
- 3-level hierarchy displayed with indentation
- All categories visible (no collapse in Phase 1)
- Inline editing (click to edit)
- "Add New" input at top of list
- Delete icon next to each item
- Maximum 3 levels enforced with note displayed

**Adding Categories:**
- Input field at top shows when clicking "Add New"
- Default to top-level (no parent)
- Allow selecting parent during creation
- Can also create at top-level then drag to organize
- New categories appear at top of list
- Auto-save on Enter/blur, cancel on Escape

**Editing Categories:**
- Click category name to edit inline
- Converts to input field with Save/Cancel buttons
- Auto-save on Enter/blur, cancel on Escape
- Validation: no duplicates (case-insensitive, globally unique)
- Show inline validation errors
- Lock other items from editing while one is being edited

**Deleting Categories:**
- Delete icon next to each category
- If category in use: "Are you sure? 'Blog' is assigned to X links."
- If not in use: delete immediately without confirmation
- Deletion removes category from all links

**Renaming Categories:**
- Does not change primary ID
- All links remain associated with renamed category

**Drag-and-Drop:**
- Move categories between parent levels (re-parenting)
- No reordering at same level in Phase 1
- Ghost/preview of dragged item
- Highlight valid drop zones in different color
- Prevent dropping at level 4: show red highlight/border with tooltip explaining why invalid
- Prevent circular references (parent under its own child)

**Validation:**
- Names must be globally unique (case-insensitive)
- Allow special characters but restrict control characters
- No length limits in Phase 1
- Maximum 3 levels enforced

### 4. Language Management

**Navigation:**
- "Languages" menu item

**Page Layout:**
- Flat list with inline editing
- "Add New" input at top
- Delete icon next to each item
- Same visual design as other management pages

**Initial Data:**
20 predefined languages (see Data Model section)

**Adding Languages:**
- Input at top, click "Add New"
- New languages appear at top
- Auto-save on Enter/blur, cancel on Escape

**Editing Languages:**
- Click to edit inline
- Save/Cancel buttons
- Validation: no duplicates (case-insensitive)
- Show inline validation errors

**Deleting Languages:**
- Delete icon next to each
- If in use: "Are you sure? 'Python' is assigned to X links."
- If not in use: delete immediately

**Free-text Entry:**
- Users can add any language name
- Not restricted to predefined list

### 5. License Management

**Navigation:**
- "Licenses" menu item

**Page Layout:**
- Flat list with inline editing
- "Add New" input at top
- Delete icon next to each item
- Display: acronym (e.g., "MIT")
- Management screen shows full name

**Initial Data:**
20 predefined licenses (see Data Model section)

**Adding Licenses:**
- Input at top for acronym
- New licenses appear at top
- Auto-save on Enter/blur, cancel on Escape

**Editing Licenses:**
- Click to edit inline
- Save/Cancel buttons
- Validation: no duplicates (case-insensitive)
- Show inline validation errors

**Deleting Licenses:**
- Delete icon next to each
- If in use: "Are you sure? 'MIT' is assigned to X links."
- If not in use: delete immediately

**Phase 1 Limitations:**
- Static text only (no links to external sites)
- Links to opensource.org or choosealicense.com in Phase 2

### 6. Tag Management

**Navigation:**
- "Tags" menu item

**Page Layout:**
- Flat list with inline editing
- "Add New" input at top
- Delete icon next to each item
- Same visual design as other management pages

**Adding Tags:**
- Input at top, click "Add New"
- New tags appear at top
- Auto-save on Enter/blur, cancel on Escape
- Autocomplete when adding tags to links (based on existing tags)

**Editing Tags:**
- Click to edit inline
- Save/Cancel buttons
- Validation: no duplicates (case-insensitive)
- Show inline validation errors

**Deleting Tags:**
- Delete icon next to each
- If in use: "Are you sure? 'tutorial' is assigned to X links."
- If not in use: delete immediately

### 7. Scheduled Updates

**Job Configuration:**
- Runs hourly as background task within main application
- Skip running until links are added
- Environment variable: `UPDATE_INTERVAL_DAYS` (default: 30, minimum: 1)
- Requires application restart to apply changes

**Update Schedule Calculation:**
- First refresh: `created_at` + `UPDATE_INTERVAL_DAYS` ± 20% random variance
- Subsequent: `refreshed_at` + `UPDATE_INTERVAL_DAYS` ± 20% random variance
- Variance recalculated after each update (keeps schedule randomized)
- Variance formula: ± 20% of UPDATE_INTERVAL_DAYS

**Batch Processing:**
- Process all links where current_time ≥ scheduled_update_time
- Minimum batch size: (number of links / interval in days / 24 hours) × 2
- Process whatever is available (could be 0)

**Update Actions:**
Monthly refresh of:
1. Logo
2. URL title
3. Description
4. Source code link (if user hasn't manually set)
5. Documentation link (if user hasn't manually set)
6. GitHub stars
7. GitHub archived status
8. GitHub last commit date
9. GitHub languages (if user hasn't manually set)
10. GitHub licenses (if user hasn't manually set)

**Update Behavior:**
- Only update database if values changed
- Log changes and errors
- Save partial data if some fetches fail
- Continue processing other links if one fails
- If main URL inaccessible: update status to "inaccessible"
- If GitHub repo inaccessible: update status to "repo_unavailable", keep last known values
- No UI notification for background job failures

**Logging:**
- Log all bookmarks processed to stdout (JSON format)
- Include appropriate error messages
- Log HTTP requests (responses not needed)
- Structured logging with standard format/layout
- Do not log API keys

**Rate Limiting:**
- Spread updates throughout month via randomized scheduling
- 2x multiplier on minimum batch size helps smooth distribution
- No manual rate limiting between requests

---

## User Interface

### Navigation

**Menu Structure:**
- Links (main table view)
- Categories
- Languages
- Licenses
- Tags
- Logout

**Menu Display:**
- Web: links across top
- Mobile: hamburger menu
- "Rusty Links" logo in menu (visible on all pages)

**Logout:**
- Button in menu
- No confirmation
- Logs out current session
- Redirects to login page

### Branding

**Name:** Rusty Links  
**Logo:** Rusty chain links icon  
**Favicon:** Rusty chain links  
**Color Scheme:** Professional/muted rust tones, beautiful colors  
**Page Titles:** "Rusty Links" (not section-specific)

### Responsive Design

**Desktop:**
- Full-width table utilizing entire viewport
- All features accessible

**Mobile:**
- Full functionality in responsive manner
- Table viewable in landscape mode
- Horizontal scroll enabled
- Hamburger menu
- No special mobile workarounds in Phase 1

**Tablet:**
- Full table with all columns
- Standard responsive behavior

### Visual Design

**Tables:**
- Clean, professional design
- Full-width (no narrow columns on large monitors)
- Sortable columns with arrow indicators
- Responsive truncation with ellipsis

**Forms:**
- Standard input fields for editable content
- Different background color for read-only fields
- Clear visual distinction between editable and non-editable
- Inline validation with error messages

**Modals:**
- Maximize modal size
- Scrollable content when needed
- No animations
- Close on X, Escape, backdrop click (with unsaved changes warning)

**Badges/Chips:**
- Used for: tags, categories, languages, licenses, status
- Color-coded for status (active/archived/inaccessible/repo_unavailable)
- X icon for removable selections
- Outlined style for suggestions vs. solid for selected

**Buttons:**
- Primary button for "Add Link" (bold color, icon + text)
- Standard Save/Cancel buttons in modals
- Delete icons next to items in management pages
- Refresh button in GitHub section (icon + text)

**Empty States:**
- "No links found" for empty table
- Simple text, no illustrations

**Loading States:**
- Spinner per field during async fetch
- Progress indicator for overall status
- "Saving..." / "Saved" indicators for auto-save (removed in favor of explicit Save)

### Date/Time Display

**Format:**
- ISO format: YYYY-MM-DD
- User's locale and timezone
- No time component shown (dates only)
- All timestamps use same format

---

## Environment Variables

**Required:**
- `DATABASE_URL` - PostgreSQL connection string
- `APP_PORT` - Web server port

**Optional:**
- `UPDATE_INTERVAL_DAYS` - Update interval (default: 30, minimum: 1, integer only)
- `RUST_LOG` - Log level (or whatever logging library uses)

**Configuration:**
- Centralized config module in backend
- Validation on startup
- Log configuration errors clearly

---

## Docker Deployment

### Dockerfile

**Multi-stage Build:**
1. Build stage: compile Rust application with optimizations (--release)
2. Runtime stage: Alpine or Distroless base image
3. Copy binary from build stage
4. No security hardening in Phase 1

**Image Details:**
- Use latest Rust version (specific version in Phase 2)
- Standard optimization flags
- No resource limits
- Publish to GitHub Container Registry
- Image name: `rusty-links`

### Docker Compose

**File:** `compose.yml`

**Services:**
1. PostgreSQL
    - Named volume for data persistence
    - Standard PostgreSQL image
2. Application
    - Built from Dockerfile
    - Depends on PostgreSQL
    - Environment variables with examples/defaults commented out

**Configuration:**
- Bridge network between services
- No health checks in Phase 1
- Restart policy: unless-stopped
- No resource limits
- Single compose file (no dev/prod separation)

**Volumes:**
- Named volume for PostgreSQL data

---

## Testing

**Scope:**
- Comprehensive unit tests for business logic:
    - URL parsing and validation
    - Duplicate detection (domain + path)
    - GitHub language detection algorithm
    - Filter logic (OR within category, AND between categories)
    - Metadata extraction priority
- Integration tests for database operations
- API endpoint tests

**Not Included in Phase 1:**
- CI/CD pipelines
- End-to-end tests
- Code coverage requirements

**Automated Builds:**
- Docker image builds automated
- Published to GitHub Container Registry

---

## Documentation

### README.md

**Sections:**
1. Project title and brief description
2. Features list
3. Screenshots/demo
4. Prerequisites (Docker, Docker Compose)
5. Installation/Quick Start
6. Configuration (environment variables reference)
7. Usage guide
8. Development setup
9. Testing
10. Roadmap (Phase 2 features)
11. License (MIT)

**Not Included:**
- Contributing guidelines (Phase 2)
- Issue templates (user will create)

**Notes:**
- Comprehensive documentation
- Clear, step-by-step instructions
- Environment variable examples
- Impact of shorter update intervals on API rate limits
- Warning about setting UPDATE_INTERVAL_DAYS too low

---

## Error Handling

### User-Facing Messages

**Principles:**
- User-friendly but specific
- Example: "Exceeded GitHub API token usage limit"
- Distinguish between timeout, DNS failure, connection refused for network errors

**Login:**
- Generic "Invalid credentials" (don't reveal if user exists)

**Duplicate Links:**
- Show existing link details (same as details modal)

**Field Errors:**
- Show as bubble/tooltip on failed fields
- Inline validation errors during editing

**Network Errors:**
- Specific error types in messages

### System Logging

**Format:**
- Structured JSON logs
- Standard format/layout for Rust logging library

**Output:**
- All logs to stdout for Docker consumption

**Content:**
- HTTP requests (not responses)
- All bookmarks processed in update job
- Specific error messages
- Field-level failure information
- No API keys logged
- No password/sensitive data

---

## Phase 2 Roadmap

Features planned for Phase 2 (detailed separately):

- Magic link email authentication
- Multiple users with roles
- Public/private links
- User profiles with password change
- Dark mode
- Browser plugin
- Data export/import
- Advanced search operators (AND, OR, NOT)
- Contributing guidelines
- More sophisticated filtering
- Category collapse/expand
- Customizable table columns
- Remember sort/filter preferences
- Health checks
- Specific Rust version pinning
- License links to external sites
- Support for GitLab/Bitbucket APIs
- GitHub GraphQL API
- More sophisticated metadata extraction heuristics
- Security hardening
- Rate limiting
- Brute force protection
- Enhanced mobile experience
- Field-level auto-save configuration
- Admin account management

---

## Development Guidelines

### Code Organization

**Monorepo Structure:**
- Clear separation between frontend and backend
- Modular architecture with single responsibility
- Well-documented code
- Follow Rust best practices and idioms

### Modules:**
- `config` - Environment variable loading and validation
- `models` - Database models and schema
- `api` - RESTful API endpoints
- `auth` - Authentication and session management
- `scraper` - Web scraping and metadata extraction
- `github` - GitHub API client
- `scheduler` - Background update job
- `ui` - Dioxus frontend components

### Database

**Migrations:**
- Use SQLx migration system
- Run automatically on startup
- Forward-only in Phase 1
- Well-documented migration files

**Queries:**
- Compile-time checked with SQLx
- Use prepared statements
- Proper indexing for performance
- Efficient joins for multi-table queries

### API Design

**Endpoints:**
Follow RESTful conventions:
- `GET /api/links` - List links (with pagination, search, filters)
- `POST /api/links` - Create link
- `GET /api/links/:id` - Get link details
- `PUT /api/links/:id` - Update link
- `DELETE /api/links/:id` - Delete link
- Similar patterns for categories, languages, licenses, tags

**Responses:**
- Standard JSON format
- Include metadata (pagination info, total counts)
- Consistent error structure
- Appropriate HTTP status codes

### Security

**Phase 1 Basics:**
- Argon2 password hashing (recommended parameters)
- Secure session tokens
- HttpOnly, secure cookies
- SameSite attributes
- Input validation
- SQL injection prevention (via SQLx)
- XSS prevention (via Dioxus escaping)

### Performance

**Optimization:**
- Efficient database queries with proper indexing
- Connection pooling
- Async/await throughout
- Compile-time optimizations (--release)
- Lazy loading where appropriate

**Scalability Considerations:**
- Designed for single-user but architecture supports Phase 2 multi-user
- Efficient batch processing in update job
- Reasonable defaults for pagination

---

## Acceptance Criteria

Phase 1 is complete when:

1. ✅ User can create account on first run
2. ✅ User can log in with email/password
3. ✅ User can add links via button or paste
4. ✅ System extracts metadata (title, description, logo, source, docs)
5. ✅ System detects and prevents duplicate URLs (domain + path)
6. ✅ GitHub integration fetches stars, languages, license, archived, last commit
7. ✅ User can view links in sortable, paginated table
8. ✅ User can search across all fields with fuzzy matching
9. ✅ User can filter by language, license, purpose
10. ✅ User can edit links (add tags, categories, languages, licenses, notes)
11. ✅ User can delete links
12. ✅ User can manage categories (3-level hierarchy with drag-drop)
13. ✅ User can manage languages, licenses, tags
14. ✅ Scheduled job updates metadata monthly with randomized timing
15. ✅ Application deployed via Docker Compose
16. ✅ Comprehensive tests pass
17. ✅ README documentation complete
18. ✅ All error handling implemented
19. ✅ Responsive design works on desktop and mobile (landscape)
20. ✅ Session management works correctly
21. ✅ Logout functionality works
22. ✅ All validation rules enforced
23. ✅ Logging configured and working

---

## Non-Functional Requirements

### Performance
- Table loads in <2 seconds for 1000 links
- Search results appear in <500ms
- Metadata extraction completes within 30 seconds per link
- Modal opens instantly

### Reliability
- Database migrations run successfully
- Background job recovers from failures
- Partial data saved on errors
- No data loss on application restart

### Maintainability
- Well-documented code
- Modular architecture
- Comprehensive tests
- Clear error messages in logs

### Usability
- Intuitive UI following web conventions
- Clear visual feedback for actions
- Helpful error messages
- Responsive design

---

## Known Limitations (Phase 1)

1. Single user only
2. No data export/import
3. No browser plugin
4. No public/private links
5. No sharing between users
6. No password requirements
7. No dark mode
8. No remembered preferences between sessions
9. No column customization in table
10. No category collapse/expand
11. No advanced search operators
12. No user profile management
13. Limited mobile optimization (landscape for table)
14. No license links to external sites
15. GitHub only (no GitLab/Bitbucket)
16. No headless browser for JavaScript-heavy sites
17. No rate limiting or brute force protection
18. No field-specific auto-save configuration
19. Unauthenticated GitHub API (60 req/hour limit)
20. No historical tracking of changes

---

## Glossary

**Link:** A bookmarked URL with associated metadata  
**Purpose/Category:** Hierarchical classification system (max 3 levels)  
**Tag:** Free-form label for categorization  
**Language:** Programming language associated with link  
**License:** Software license associated with link  
**Status:** Current state of link (active, archived, inaccessible, repo_unavailable)  
**Metadata:** Auto-extracted information (title, description, logo, etc.)  
**Source Code URL:** Link to repository (GitHub, GitLab, etc.)  
**Documentation URL:** Link to project documentation  
**Fuzzy Search:** Search that handles typos and approximate matches  
**Debounce:** Delay after user stops typing before action triggers

---

## Contact & Support

**Repository:** https://github.com/[user]/rusty-links  
**Issues:** GitHub Issues (templates will be provided by user)  
**License:** MIT

---

*This specification is for Phase 1 only. Phase 2 features and requirements will be documented separately.*