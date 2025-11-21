# Part 3: Link Management Core (Steps 10-15)

## Context
Parts 1-2 are complete. We have:
- Database with users, links, sessions, categories, tags, languages, licenses tables
- Auth system with login/logout/session management
- Basic UI with Setup, Login, and placeholder Links pages
- Error handling framework (AppError)

---

## Step 10: Link Model

### Goal
Create the Link model with database operations for CRUD.

### Prompt

```text
Create the Link model in `src/models/link.rs` for the Rusty Links application.

The links table schema (already exists in database):
- id: UUID (primary key)
- user_id: UUID (foreign key to users)
- url: TEXT NOT NULL
- domain: TEXT NOT NULL
- path: TEXT
- title: TEXT
- description: TEXT
- logo: TEXT (favicon/logo URL)
- is_github_repo: BOOLEAN DEFAULT FALSE
- github_stars: INTEGER
- github_archived: BOOLEAN
- github_last_commit: TIMESTAMPTZ
- status: TEXT DEFAULT 'active' (active, archived, inaccessible, repo_unavailable)
- refreshed_at: TIMESTAMPTZ
- created_at: TIMESTAMPTZ DEFAULT NOW()
- updated_at: TIMESTAMPTZ DEFAULT NOW()

Implement the following in `src/models/link.rs`:

1. A `Link` struct with all fields, deriving Serialize, Deserialize, FromRow, Debug, Clone
2. A `CreateLink` struct for creating new links (url required, optional: title, description)
3. A `UpdateLink` struct for updating links (all fields optional)
4. These async functions:
   - `create(pool, user_id, create_link) -> Result<Link, AppError>` - Parse URL to extract domain/path, insert into DB
   - `get_by_id(pool, id, user_id) -> Result<Link, AppError>` - Get single link (must belong to user)
   - `get_all_by_user(pool, user_id) -> Result<Vec<Link>, AppError>` - Get all links for a user
   - `update(pool, id, user_id, update_link) -> Result<Link, AppError>` - Update link fields
   - `delete(pool, id, user_id) -> Result<(), AppError>` - Delete a link

Use SQLx for database queries. Use the existing `AppError` type for errors.
Parse the URL using the `url` crate to extract domain and path.

After creating the file, add `pub mod link;` to `src/models/mod.rs` and re-export it.
```

### Verification
- `cargo check` passes
- Link model is exported from models module

---

## Step 11: Link API - Create & Read

### Goal
Implement POST /api/links and GET /api/links endpoints.

### Prompt

```text
Create the Link API endpoints in `src/api/links.rs` for the Rusty Links application.

Implement two endpoints:

1. `POST /api/links` - Create a new link
   - Requires authentication (use existing auth middleware pattern from auth.rs)
   - Request body: `{ "url": "https://...", "title": "optional", "description": "optional" }`
   - Validate URL format
   - Call `Link::create()` from the model
   - Return 201 with the created Link JSON

2. `GET /api/links` - List all links for the authenticated user
   - Requires authentication
   - Call `Link::get_all_by_user()`
   - Return 200 with array of Link JSON

Create a `create_router(pool: PgPool) -> Router` function that sets up the routes.

Use the existing patterns from `src/api/auth.rs`:
- Extract user from session using cookies
- Use `AppError` for error responses
- Use `axum::extract::{State, Json}`
- Use `axum_extra::extract::CookieJar`

After creating the file:
1. Add `pub mod links;` to `src/api/mod.rs`
2. In `src/api/mod.rs`, nest the links router under the main API router:
   `.nest("/links", links::create_router(pool.clone()))`
```

### Verification
- `cargo check` passes
- API endpoints are wired into the main router

---

## Step 12: Link API - Update & Delete

### Goal
Implement PUT /api/links/:id and DELETE /api/links/:id endpoints.

### Prompt

```text
Add update and delete endpoints to `src/api/links.rs` for the Rusty Links application.

Add two new endpoints:

1. `PUT /api/links/:id` - Update a link
   - Requires authentication
   - Path parameter: link ID (UUID)
   - Request body: `{ "title": "optional", "description": "optional", "status": "optional" }`
   - Verify link belongs to authenticated user
   - Call `Link::update()` from the model
   - Return 200 with the updated Link JSON

2. `DELETE /api/links/:id` - Delete a link
   - Requires authentication
   - Path parameter: link ID (UUID)
   - Verify link belongs to authenticated user
   - Call `Link::delete()` from the model
   - Return 204 No Content on success

Add these routes to the existing `create_router` function:
- `.route("/:id", put(update_link).delete(delete_link))`

Use `axum::extract::Path` to extract the UUID from the path.
Handle the case where the link doesn't exist or doesn't belong to the user (return 404).
```

### Verification
- `cargo check` passes
- All four CRUD endpoints work (test with curl if possible)

---

## Step 13: Links UI - Display List

### Goal
Build the Links page to display all user links in a list/grid.

### Prompt

```text
Update the Links page in `src/ui/pages/links.rs` to display the user's links.

Replace the placeholder content with a functional link list:

1. On component mount, fetch links from `GET /api/links`
2. Display links in a clean list/card layout showing:
   - Title (or URL if no title)
   - Domain
   - Description (truncated if long)
   - Status indicator (active/archived)
   - Created date
3. Handle loading state (show "Loading...")
4. Handle empty state (show "No links yet. Add your first link!")
5. Handle error state (show error message)

Use existing patterns from the Login/Setup pages:
- `use_signal` for state management
- `use_effect` for data fetching on mount
- `spawn` for async operations
- `reqwest::Client` for HTTP requests

Add CSS classes that match the existing style.css patterns:
- `.links-container` for the main container
- `.link-card` for each link item
- `.link-title`, `.link-domain`, `.link-description` for text elements

Keep the existing Navbar component at the top.
```

### Verification
- `cargo check` passes
- Links page shows loading state, then either links or empty state

---

## Step 14: Links UI - Create Form

### Goal
Add a form to create new links with basic fields.

### Prompt

```text
Add a link creation form to the Links page in `src/ui/pages/links.rs`.

Add the following to the Links page:

1. An "Add Link" button that opens a form (can be inline or modal-style)
2. A form with fields:
   - URL (required) - text input
   - Title (optional) - text input
   - Description (optional) - textarea
3. Form validation:
   - URL must not be empty
   - URL must be a valid URL format (basic check for http:// or https://)
4. Submit handler that:
   - Shows loading state on button
   - POSTs to `/api/links`
   - On success: adds the new link to the list, clears form, hides form
   - On error: shows error message
5. Cancel button to close the form without saving

Use signals to manage:
- `show_form: bool` - whether form is visible
- `new_url: String` - URL input value
- `new_title: String` - title input value
- `new_description: String` - description input value
- `form_loading: bool` - form submission state
- `form_error: Option<String>` - form error message

Add appropriate CSS classes:
- `.link-form` for the form container
- `.form-actions` for button row
```

### Verification
- `cargo check` passes
- Can add a new link and see it appear in the list

---

## Step 15: Links UI - Edit & Delete

### Goal
Add edit and delete functionality to the Links UI.

### Prompt

```text
Add edit and delete functionality to the Links page in `src/ui/pages/links.rs`.

Add the following features:

1. **Delete Link**
   - Add a delete button (trash icon or "Delete" text) to each link card
   - On click, show confirmation (can be simple browser confirm or inline)
   - Call `DELETE /api/links/:id`
   - On success, remove the link from the list
   - On error, show error message

2. **Edit Link**
   - Add an edit button to each link card
   - On click, show edit form (can reuse create form with pre-filled values)
   - Form shows current title, description, and status
   - Add a status dropdown: active, archived
   - Submit calls `PUT /api/links/:id`
   - On success, update the link in the list, close form
   - On error, show error message

3. **State Management**
   - Add `editing_link_id: Option<Uuid>` signal to track which link is being edited
   - When editing, the form shows "Edit Link" instead of "Add Link"
   - Cancel editing returns to normal view

4. **Visual Feedback**
   - Show loading spinner on delete/edit buttons during operation
   - Disable buttons during operations to prevent double-clicks

Refactor if needed to keep the component manageable. Consider extracting:
- `LinkCard` component for displaying a single link
- `LinkForm` component for create/edit form

Add CSS for edit/delete buttons:
- `.link-actions` container for action buttons
- `.btn-icon` for icon buttons
- `.btn-danger` for delete button styling
```

### Verification
- `cargo check` passes
- Can edit link title/description/status
- Can delete links with confirmation
- UI updates immediately after operations
