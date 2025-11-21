# Part 5: Metadata & Languages/Licenses (Steps 22-26)

## Context
Part 4 is complete. We have:
- Category model with hierarchical support
- Tag model and API
- Link-category and link-tag associations
- UI components for selecting categories and tags

---

## Step 22: Language & License Models

### Goal
Models for programming languages and software licenses.

### Prompt

````text
Create Language and License models for the Rusty Links application.

**Part 1: Language model in `src/models/language.rs`**

The languages table schema (already exists):
- id: UUID (primary key)
- name: TEXT NOT NULL
- user_id: UUID (nullable - NULL means global/seeded)
- created_at: TIMESTAMPTZ DEFAULT NOW()

Note: 20 global languages are pre-seeded (Rust, Python, JavaScript, etc.)

Implement:
1. `Language` struct with all fields
2. Functions:
   - `get_all_available(pool, user_id) -> Result<Vec<Language>, AppError>`
     - Returns global languages (user_id IS NULL) + user's custom languages
   - `create(pool, user_id, name) -> Result<Language, AppError>`
     - Creates a user-specific language
   - `delete(pool, id, user_id) -> Result<(), AppError>`
     - Only delete user-created languages (not global)

**Part 2: License model in `src/models/license.rs`**

The licenses table schema (already exists):
- id: UUID (primary key)
- name: TEXT NOT NULL
- user_id: UUID (nullable - NULL means global/seeded)
- created_at: TIMESTAMPTZ DEFAULT NOW()

Note: 20 global licenses are pre-seeded (MIT, Apache-2.0, GPL-3.0, etc.)

Implement same pattern as Language:
1. `License` struct
2. Functions: `get_all_available`, `create`, `delete`

Add to `src/models/mod.rs`:
```rust
pub mod language;
pub mod license;
```
````

### Verification
- `cargo check` passes
- Models are exported

---

## Step 23: Language & License API

### Goal
Endpoints for listing and managing languages/licenses.

### Prompt

````text
Create Language and License API endpoints for the Rusty Links application.

**Part 1: `src/api/languages.rs`**

1. `GET /api/languages` - List available languages
   - Returns global + user's custom languages
   - Each item has `is_global: bool` field (true if user_id is null)

2. `POST /api/languages` - Create custom language
   - Body: `{ "name": "..." }`
   - Returns created Language

3. `DELETE /api/languages/:id` - Delete custom language
   - Returns 403 if trying to delete global language
   - Returns 204 on success

**Part 2: `src/api/licenses.rs`**

Same pattern as languages:
1. `GET /api/licenses` - List available licenses
2. `POST /api/licenses` - Create custom license
3. `DELETE /api/licenses/:id` - Delete custom license

Wire into `src/api/mod.rs`:
```rust
.nest("/languages", languages::create_router(pool.clone()))
.nest("/licenses", licenses::create_router(pool.clone()))
```
````

### Verification
- `cargo check` passes
- Can list global languages/licenses
- Can create custom languages/licenses

---

## Step 24: Link-Language & Link-License Association

### Goal
API endpoints to assign languages/licenses to links.

### Prompt

```text
Add language and license association to links for the Rusty Links application.

**Part 1: Add to Link model (`src/models/link.rs`)**

The junction tables (already exist):
- link_languages: link_id, language_id
- link_licenses: link_id, license_id

Add functions:
- `add_language(pool, link_id, language_id, user_id) -> Result<(), AppError>`
- `remove_language(pool, link_id, language_id, user_id) -> Result<(), AppError>`
- `get_languages(pool, link_id, user_id) -> Result<Vec<Language>, AppError>`
- `add_license(pool, link_id, license_id, user_id) -> Result<(), AppError>`
- `remove_license(pool, link_id, license_id, user_id) -> Result<(), AppError>`
- `get_licenses(pool, link_id, user_id) -> Result<Vec<License>, AppError>`

**Part 2: Add endpoints to `src/api/links.rs`**

Languages:
1. `POST /api/links/:id/languages` - Body: `{ "language_id": "uuid" }`
2. `DELETE /api/links/:id/languages/:language_id`
3. `GET /api/links/:id/languages`

Licenses:
1. `POST /api/links/:id/licenses` - Body: `{ "license_id": "uuid" }`
2. `DELETE /api/links/:id/licenses/:license_id`
3. `GET /api/links/:id/licenses`

**Part 3: Update LinkWithMetadata**

Add languages and licenses to the LinkWithMetadata struct.
Update `GET /api/links` to include all metadata (categories, tags, languages, licenses).
```

### Verification
- `cargo check` passes
- Can assign languages/licenses to links

---

## Step 25: Metadata Display UI

### Goal
Show categories, tags, languages, licenses on link cards.

### Prompt

```text
Update link cards to display all metadata in `src/ui/pages/links.rs`.

**Part 1: Update link card display**

For each link card, show:
1. Categories - colored badges (e.g., blue)
2. Tags - colored badges (e.g., green)
3. Languages - colored badges (e.g., purple) with optional icon
4. Licenses - colored badges (e.g., orange)

Layout suggestion:
```
┌─────────────────────────────────────┐
│ Title                    [Edit][Del]│
│ domain.com                          │
│ Description text here...            │
│                                     │
│ [Category1] [Category2]             │
│ #tag1 #tag2 #tag3                   │
│ 🦀 Rust  📜 MIT                     │
└─────────────────────────────────────┘
```

**Part 2: Create `src/ui/components/metadata_badges.rs`**

A reusable component that displays metadata badges:
- Props: categories, tags, languages, licenses (all Vec)
- Renders badges with appropriate colors
- Handles empty states (don't show section if empty)

**Part 3: Update data fetching**

Ensure `GET /api/links` returns full metadata.
Update the links page to use the complete link data.

Add CSS:
- `.metadata-row` for each metadata type row
- `.badge-category`, `.badge-tag`, `.badge-language`, `.badge-license`
```

### Verification
- `cargo check` passes
- Link cards show all assigned metadata

---

## Step 26: Metadata Assignment UI

### Goal
UI for assigning metadata to links (dropdowns, multi-select).

### Prompt

```text
Create comprehensive metadata assignment UI for the Rusty Links application.

**Part 1: Create `src/ui/components/language_select.rs`**

Similar to category_select:
- Props: `selected_ids: Vec<Uuid>`, `on_change: EventHandler<Vec<Uuid>>`
- Fetches languages from `/api/languages`
- Multi-select dropdown
- Option to create new custom language inline

**Part 2: Create `src/ui/components/license_select.rs`**

Similar pattern:
- Fetches licenses from `/api/licenses`
- Multi-select (though typically single license per link)
- Option to create new custom license inline

**Part 3: Update link form in `src/ui/pages/links.rs`**

Add all four metadata selectors to the create/edit form:
1. CategorySelect - for categories
2. TagSelect - for tags
3. LanguageSelect - for programming languages
4. LicenseSelect - for licenses

Form layout:
```
URL: [________________]
Title: [________________]
Description: [________________]

Categories: [Select categories...]
Tags: [Select or create tags...]
Languages: [Select languages...]
License: [Select license...]

[Cancel] [Save]
```

**Part 4: Handle metadata on save**

When saving a link (create or edit):
1. Create/update the link first
2. Sync categories (add new, remove deselected)
3. Sync tags
4. Sync languages
5. Sync licenses

Create helper function `sync_metadata(link_id, old_ids, new_ids, add_fn, remove_fn)`.

**Part 5: Export components in `src/ui/components/mod.rs`**
```rust
pub mod language_select;
pub mod license_select;
```
```

### Verification
- `cargo check` passes
- Can assign all metadata types when creating/editing links
- Metadata persists and displays correctly
