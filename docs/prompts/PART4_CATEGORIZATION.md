# Part 4: Categorization System (Steps 16-21)

## Context
Part 3 is complete. We have:
- Link model with CRUD operations
- Link API endpoints (create, read, update, delete)
- Links UI page with list, create, edit, delete functionality

---

## Step 16: Category Model

### Goal
Create the Category model with hierarchical support (3-level depth).

### Prompt

```text
Create the Category model in `src/models/category.rs` for the Rusty Links application.

The categories table schema (already exists):
- id: UUID (primary key)
- user_id: UUID (foreign key to users)
- name: TEXT NOT NULL
- parent_id: UUID (nullable, foreign key to categories for hierarchy)
- depth: INTEGER NOT NULL DEFAULT 0 (0=root, 1=child, 2=grandchild, max depth is 2)
- created_at: TIMESTAMPTZ DEFAULT NOW()

Implement the following:

1. A `Category` struct with all fields, deriving Serialize, Deserialize, FromRow, Debug, Clone
2. A `CreateCategory` struct: name (required), parent_id (optional)
3. A `CategoryWithChildren` struct for hierarchical display (category + Vec of children)

4. These async functions:
   - `create(pool, user_id, create_category) -> Result<Category, AppError>`
     - If parent_id provided, validate parent exists and belongs to user
     - Calculate depth from parent (parent.depth + 1)
     - Reject if depth would exceed 2
   - `get_by_id(pool, id, user_id) -> Result<Category, AppError>`
   - `get_all_by_user(pool, user_id) -> Result<Vec<Category>, AppError>` - Flat list
   - `get_tree_by_user(pool, user_id) -> Result<Vec<CategoryWithChildren>, AppError>` - Hierarchical tree
   - `update(pool, id, user_id, name) -> Result<Category, AppError>` - Only name can be updated
   - `delete(pool, id, user_id) -> Result<(), AppError>` - Also delete children (cascade)

Add to `src/models/mod.rs`: `pub mod category;`
```

### Verification
- `cargo check` passes
- Category model is exported

---

## Step 17: Category API

### Goal
CRUD endpoints for categories with parent relationship support.

### Prompt

```text
Create Category API endpoints in `src/api/categories.rs` for the Rusty Links application.

Implement these endpoints:

1. `POST /api/categories` - Create a category
   - Request body: `{ "name": "...", "parent_id": "optional uuid" }`
   - Returns 201 with created Category

2. `GET /api/categories` - List all categories (flat)
   - Returns array of Category objects

3. `GET /api/categories/tree` - Get category tree (hierarchical)
   - Returns array of CategoryWithChildren objects

4. `GET /api/categories/:id` - Get single category
   - Returns Category or 404

5. `PUT /api/categories/:id` - Update category name
   - Request body: `{ "name": "..." }`
   - Returns updated Category

6. `DELETE /api/categories/:id` - Delete category and children
   - Returns 204

All endpoints require authentication.

Create `create_router(pool: PgPool) -> Router` and wire it into `src/api/mod.rs`:
`.nest("/categories", categories::create_router(pool.clone()))`
```

### Verification
- `cargo check` passes
- Category endpoints accessible under /api/categories

---

## Step 18: Tag Model & API

### Goal
Create Tag model and CRUD endpoints.

### Prompt

```text
Create the Tag model and API for the Rusty Links application.

**Part 1: Model in `src/models/tag.rs`**

The tags table schema (already exists):
- id: UUID (primary key)
- user_id: UUID (foreign key)
- name: TEXT NOT NULL
- created_at: TIMESTAMPTZ DEFAULT NOW()

Implement:
1. `Tag` struct with all fields
2. `CreateTag` struct with name field
3. Functions:
   - `create(pool, user_id, name) -> Result<Tag, AppError>`
   - `get_by_id(pool, id, user_id) -> Result<Tag, AppError>`
   - `get_all_by_user(pool, user_id) -> Result<Vec<Tag>, AppError>`
   - `delete(pool, id, user_id) -> Result<(), AppError>`

Add to `src/models/mod.rs`: `pub mod tag;`

**Part 2: API in `src/api/tags.rs`**

Endpoints:
1. `POST /api/tags` - Create tag, body: `{ "name": "..." }`
2. `GET /api/tags` - List all user tags
3. `DELETE /api/tags/:id` - Delete tag

Wire into `src/api/mod.rs`: `.nest("/tags", tags::create_router(pool.clone()))`
```

### Verification
- `cargo check` passes
- Tag endpoints accessible under /api/tags

---

## Step 19: Link-Category Association

### Goal
API endpoints to assign/remove categories from links.

### Prompt

```text
Add category association endpoints to the links API in `src/api/links.rs`.

The link_categories junction table (already exists):
- link_id: UUID (foreign key)
- category_id: UUID (foreign key)
- PRIMARY KEY (link_id, category_id)

**Part 1: Add to Link model (`src/models/link.rs`)**

Add functions:
- `add_category(pool, link_id, category_id, user_id) -> Result<(), AppError>`
  - Verify link belongs to user
  - Verify category belongs to user
  - Insert into link_categories (ignore if already exists)
- `remove_category(pool, link_id, category_id, user_id) -> Result<(), AppError>`
- `get_categories(pool, link_id, user_id) -> Result<Vec<Category>, AppError>`

**Part 2: Add endpoints to `src/api/links.rs`**

1. `POST /api/links/:id/categories` - Add category to link
   - Body: `{ "category_id": "uuid" }`
   - Returns 200 with updated list of categories

2. `DELETE /api/links/:id/categories/:category_id` - Remove category from link
   - Returns 200 with updated list of categories

3. `GET /api/links/:id/categories` - Get link's categories
   - Returns array of Category

Update the `GET /api/links` response to include categories for each link.
Consider creating a `LinkWithMetadata` struct that includes categories.
```

### Verification
- `cargo check` passes
- Can add/remove categories from links via API

---

## Step 20: Link-Tag Association

### Goal
API endpoints to assign/remove tags from links.

### Prompt

```text
Add tag association endpoints to the links API in `src/api/links.rs`.

The link_tags junction table (already exists):
- link_id: UUID (foreign key)
- tag_id: UUID (foreign key)
- PRIMARY KEY (link_id, tag_id)

**Part 1: Add to Link model (`src/models/link.rs`)**

Add functions:
- `add_tag(pool, link_id, tag_id, user_id) -> Result<(), AppError>`
- `remove_tag(pool, link_id, tag_id, user_id) -> Result<(), AppError>`
- `get_tags(pool, link_id, user_id) -> Result<Vec<Tag>, AppError>`

**Part 2: Add endpoints to `src/api/links.rs`**

1. `POST /api/links/:id/tags` - Add tag to link
   - Body: `{ "tag_id": "uuid" }`
   - Returns 200 with updated list of tags

2. `DELETE /api/links/:id/tags/:tag_id` - Remove tag from link
   - Returns 200 with updated list of tags

3. `GET /api/links/:id/tags` - Get link's tags
   - Returns array of Tag

Update `LinkWithMetadata` to also include tags.
Update `GET /api/links` to include tags for each link.
```

### Verification
- `cargo check` passes
- Can add/remove tags from links via API

---

## Step 21: Categories & Tags UI

### Goal
UI components for managing and assigning categories/tags.

### Prompt

```text
Add category and tag management UI to the Rusty Links application.

**Part 1: Create `src/ui/components/category_select.rs`**

A reusable component for selecting categories:
- Props: `selected_ids: Vec<Uuid>`, `on_change: EventHandler<Vec<Uuid>>`
- Fetches categories from `/api/categories/tree` on mount
- Displays hierarchical dropdown/tree selector
- Allows multiple selection
- Shows selected categories as chips/tags

**Part 2: Create `src/ui/components/tag_select.rs`**

A reusable component for selecting/creating tags:
- Props: `selected_ids: Vec<Uuid>`, `on_change: EventHandler<Vec<Uuid>>`
- Fetches tags from `/api/tags` on mount
- Displays as multi-select with search
- Allows creating new tags inline (POST to /api/tags)
- Shows selected tags as chips

**Part 3: Update `src/ui/pages/links.rs`**

In the link form (create/edit):
1. Add CategorySelect component below description
2. Add TagSelect component below categories
3. On form submit, after creating/updating link:
   - Add selected categories via POST /api/links/:id/categories
   - Add selected tags via POST /api/links/:id/tags

In the link card:
- Display category names as small badges
- Display tag names as small badges (different color)

**Part 4: Update `src/ui/components/mod.rs`**

Export the new components:
```rust
pub mod category_select;
pub mod tag_select;
```

Add CSS classes:
- `.category-badge`, `.tag-badge` for metadata badges
- `.multi-select` for the selection components
- `.chip` for selected item chips
```

### Verification
- `cargo check` passes
- Can assign categories and tags when creating/editing links
- Categories and tags display on link cards
