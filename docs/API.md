# Rusty Links API Documentation

Complete RESTful API reference for managing bookmarks, categories, tags, and metadata.

## Base URL

```
http://localhost:8080/api
```

For production deployments, replace with your actual domain.

## Authentication

Session-based authentication using HTTP-only cookies. Login via `/api/auth/login` to obtain a session cookie, which will be automatically sent with subsequent requests.

**Authentication Flow:**
1. Check if setup is required: `GET /api/auth/check-setup`
2. If setup required, create first user: `POST /api/auth/setup`
3. Otherwise, login: `POST /api/auth/login`
4. Session cookie is set automatically
5. Use authenticated endpoints
6. Logout: `POST /api/auth/logout`

## Response Format

All endpoints return JSON. Successful responses have 2XX status codes.

### Success Response

```json
{
  "id": "123e4567-e89b-12d3-a456-426614174000",
  "title": "Example Link",
  "url": "https://example.com"
}
```

### Error Response

```json
{
  "error": "Link not found"
}
```

### HTTP Status Codes

| Code | Meaning |
|------|---------|
| 200 | OK - Request successful |
| 201 | Created - Resource created successfully |
| 204 | No Content - Request successful, no content to return |
| 400 | Bad Request - Invalid request data |
| 401 | Unauthorized - Authentication required or failed |
| 403 | Forbidden - Authenticated but not authorized |
| 404 | Not Found - Resource not found |
| 409 | Conflict - Resource already exists |
| 500 | Internal Server Error - Server error |
| 503 | Service Unavailable - Service temporarily unavailable |

## Rate Limiting

No rate limiting for authenticated users. External API calls (GitHub, web scraping) are rate-limited internally to respect third-party service limits.

---

## Table of Contents

- [Authentication](#authentication-endpoints)
- [Links](#links-endpoints)
- [Categories](#categories-endpoints)
- [Tags](#tags-endpoints)
- [Languages](#languages-endpoints)
- [Licenses](#licenses-endpoints)
- [Scraping](#scraping-endpoints)
- [Health](#health-endpoints)
- [Complete Examples](#complete-examples)

---

## Authentication Endpoints

### Check Setup Status

Check if initial application setup is required.

**Endpoint:** `GET /api/auth/check-setup`

**Authentication:** None required

**Response:** 200 OK

```json
{
  "setup_required": true
}
```

**Example:**

```bash
curl http://localhost:8080/api/auth/check-setup
```

---

### Create First User

Create the first user account during initial application setup.

**Endpoint:** `POST /api/auth/setup`

**Authentication:** None required (only works if no users exist)

**Request Body:**

```json
{
  "email": "admin@example.com",
  "password": "secure_password"
}
```

**Response:** 201 Created

```json
{
  "id": "123e4567-e89b-12d3-a456-426614174000",
  "email": "admin@example.com",
  "created_at": "2024-01-15T10:30:00Z"
}
```

**Errors:**
- 403 Forbidden - Setup already completed
- 400 Bad Request - Invalid email or password
- 409 Conflict - Email already exists

**Example:**

```bash
curl -X POST http://localhost:8080/api/auth/setup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@example.com",
    "password": "SecurePass123!"
  }'
```

---

### Login

Authenticate with email and password.

**Endpoint:** `POST /api/auth/login`

**Authentication:** None required

**Request Body:**

```json
{
  "email": "user@example.com",
  "password": "user_password"
}
```

**Response:** 200 OK

```json
{
  "id": "123e4567-e89b-12d3-a456-426614174000",
  "email": "user@example.com",
  "created_at": "2024-01-15T10:30:00Z"
}
```

Sets `session_id` cookie (HTTP-only, secure in production).

**Errors:**
- 401 Unauthorized - Invalid email or password

**Example:**

```bash
curl -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -c cookies.txt \
  -d '{
    "email": "user@example.com",
    "password": "password123"
  }'
```

---

### Get Current User

Get information about the currently authenticated user.

**Endpoint:** `GET /api/auth/me`

**Authentication:** Required

**Response:** 200 OK

```json
{
  "id": "123e4567-e89b-12d3-a456-426614174000",
  "email": "user@example.com",
  "created_at": "2024-01-15T10:30:00Z"
}
```

**Errors:**
- 401 Unauthorized - No valid session

**Example:**

```bash
curl http://localhost:8080/api/auth/me \
  -b cookies.txt
```

---

### Logout

End the current session and clear the session cookie.

**Endpoint:** `POST /api/auth/logout`

**Authentication:** Required

**Response:** 200 OK

```json
{
  "message": "Logged out successfully"
}
```

**Errors:**
- 401 Unauthorized - No valid session

**Example:**

```bash
curl -X POST http://localhost:8080/api/auth/logout \
  -b cookies.txt \
  -c cookies.txt
```

---

## Links Endpoints

### Create Link

Create a new link with automatic metadata extraction.

**Endpoint:** `POST /api/links`

**Authentication:** Required

**Request Body:**

```json
{
  "url": "https://example.com/page",
  "title": "Optional Title",
  "description": "Optional description",
  "logo": "https://example.com/logo.png",
  "category_ids": ["uuid-1", "uuid-2"],
  "tag_ids": ["uuid-3"],
  "language_ids": ["uuid-4"],
  "license_ids": ["uuid-5"]
}
```

**Required Fields:**
- `url` - Valid HTTP/HTTPS URL

**Optional Fields:**
- `title` - Link title (auto-extracted if not provided)
- `description` - Link description (auto-extracted if not provided)
- `logo` - Logo URL (auto-extracted if not provided)
- `category_ids` - Array of category UUIDs
- `tag_ids` - Array of tag UUIDs
- `language_ids` - Array of language UUIDs
- `license_ids` - Array of license UUIDs

**Response:** 201 Created

```json
{
  "id": "123e4567-e89b-12d3-a456-426614174000",
  "url": "https://example.com/page",
  "title": "Example Page",
  "description": "An example website",
  "logo": "https://example.com/favicon.ico",
  "user_id": "user-uuid",
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:30:00Z",
  "last_metadata_update": "2024-01-15T10:30:00Z",
  "github_stars": null,
  "github_language": null
}
```

**GitHub Repositories:**

For GitHub URLs, additional metadata is automatically fetched:
- Stars count
- Primary language
- License
- Topics (converted to tags)

**Errors:**
- 400 Bad Request - Invalid URL format
- 401 Unauthorized - No valid session

**Example:**

```bash
curl -X POST http://localhost:8080/api/links \
  -H "Content-Type: application/json" \
  -b cookies.txt \
  -d '{
    "url": "https://github.com/rust-lang/rust",
    "category_ids": ["category-uuid"]
  }'
```

---

### List Links

Get all links for the authenticated user with optional filtering and pagination.

**Endpoint:** `GET /api/links`

**Authentication:** Required

**Query Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `search` | string | Search in title, description, URL |
| `category_id` | UUID | Filter by category |
| `tag_id` | UUID | Filter by tag |
| `language_id` | UUID | Filter by language |
| `license_id` | UUID | Filter by license |
| `limit` | integer | Number of results (default: 50, max: 100) |
| `offset` | integer | Pagination offset (default: 0) |
| `sort` | string | Sort field: `created_at`, `updated_at`, `title`, `url` (default: `created_at`) |
| `order` | string | Sort order: `asc` or `desc` (default: `desc`) |

**Response:** 200 OK

```json
{
  "links": [
    {
      "id": "123e4567-e89b-12d3-a456-426614174000",
      "url": "https://example.com",
      "title": "Example",
      "description": "An example website",
      "logo": "https://example.com/favicon.ico",
      "user_id": "user-uuid",
      "created_at": "2024-01-15T10:30:00Z",
      "updated_at": "2024-01-15T10:30:00Z",
      "last_metadata_update": "2024-01-15T10:30:00Z",
      "github_stars": 1000,
      "github_language": "Rust",
      "categories": [
        {
          "id": "cat-uuid",
          "name": "Development",
          "level": 1
        }
      ]
    }
  ],
  "total": 42,
  "limit": 50,
  "offset": 0
}
```

**Example:**

```bash
# List all links
curl http://localhost:8080/api/links \
  -b cookies.txt

# Search and filter
curl "http://localhost:8080/api/links?search=rust&category_id=uuid&limit=10&sort=title&order=asc" \
  -b cookies.txt
```

---

### Update Link

Update an existing link.

**Endpoint:** `PUT /api/links/:id`

**Authentication:** Required

**Request Body:**

```json
{
  "url": "https://example.com/updated",
  "title": "Updated Title",
  "description": "Updated description",
  "logo": "https://example.com/new-logo.png"
}
```

All fields are optional. Only provided fields will be updated.

**Response:** 200 OK

Returns the updated link object.

**Errors:**
- 404 Not Found - Link not found
- 401 Unauthorized - No valid session
- 403 Forbidden - Link belongs to another user

**Example:**

```bash
curl -X PUT http://localhost:8080/api/links/123e4567-e89b-12d3-a456-426614174000 \
  -H "Content-Type: application/json" \
  -b cookies.txt \
  -d '{
    "title": "New Title"
  }'
```

---

### Delete Link

Delete a link.

**Endpoint:** `DELETE /api/links/:id`

**Authentication:** Required

**Response:** 204 No Content

**Errors:**
- 404 Not Found - Link not found
- 401 Unauthorized - No valid session
- 403 Forbidden - Link belongs to another user

**Example:**

```bash
curl -X DELETE http://localhost:8080/api/links/123e4567-e89b-12d3-a456-426614174000 \
  -b cookies.txt
```

---

### Refresh Link Metadata

Refresh metadata for a specific link by re-scraping the URL.

**Endpoint:** `POST /api/links/:id/refresh`

**Authentication:** Required

**Response:** 200 OK

Returns the updated link object with refreshed metadata.

**Example:**

```bash
curl -X POST http://localhost:8080/api/links/123e4567-e89b-12d3-a456-426614174000/refresh \
  -b cookies.txt
```

---

### Refresh GitHub Metadata

Refresh GitHub-specific metadata for a GitHub repository link.

**Endpoint:** `POST /api/links/:id/refresh-github`

**Authentication:** Required

**Response:** 200 OK

Returns the updated link object with refreshed GitHub data (stars, language, license, etc.).

**Errors:**
- 400 Bad Request - Link is not a GitHub repository

**Example:**

```bash
curl -X POST http://localhost:8080/api/links/123e4567-e89b-12d3-a456-426614174000/refresh-github \
  -b cookies.txt
```

---

### Export Links

Export all links as JSON.

**Endpoint:** `GET /api/links/export`

**Authentication:** Required

**Response:** 200 OK

```json
{
  "version": "1.0",
  "exported_at": "2024-01-15T10:30:00Z",
  "links": [
    {
      "url": "https://example.com",
      "title": "Example",
      "description": "Description",
      "categories": ["Development", "Rust"],
      "tags": ["web", "framework"],
      "languages": ["Rust"],
      "licenses": ["MIT"]
    }
  ]
}
```

**Example:**

```bash
curl http://localhost:8080/api/links/export \
  -b cookies.txt \
  -o links_backup.json
```

---

### Import Links

Import links from JSON file.

**Endpoint:** `POST /api/links/import`

**Authentication:** Required

**Request Body:**

Same format as export. Accepts both full export format and simplified array format.

**Response:** 200 OK

```json
{
  "imported": 42,
  "skipped": 3,
  "errors": []
}
```

**Example:**

```bash
curl -X POST http://localhost:8080/api/links/import \
  -H "Content-Type: application/json" \
  -b cookies.txt \
  -d @links_backup.json
```

---

### Bulk Delete Links

Delete multiple links at once.

**Endpoint:** `POST /api/links/bulk/delete`

**Authentication:** Required

**Request Body:**

```json
{
  "link_ids": [
    "uuid-1",
    "uuid-2",
    "uuid-3"
  ]
}
```

**Response:** 200 OK

```json
{
  "deleted": 3
}
```

**Example:**

```bash
curl -X POST http://localhost:8080/api/links/bulk/delete \
  -H "Content-Type: application/json" \
  -b cookies.txt \
  -d '{
    "link_ids": ["uuid-1", "uuid-2"]
  }'
```

---

### Bulk Update Categories

Assign categories to multiple links at once.

**Endpoint:** `POST /api/links/bulk/categories`

**Authentication:** Required

**Request Body:**

```json
{
  "link_ids": ["uuid-1", "uuid-2"],
  "category_ids": ["cat-uuid-1", "cat-uuid-2"],
  "replace": false
}
```

**Parameters:**
- `replace` - If true, replaces existing categories. If false, adds to existing categories.

**Response:** 200 OK

```json
{
  "updated": 2
}
```

**Example:**

```bash
curl -X POST http://localhost:8080/api/links/bulk/categories \
  -H "Content-Type: application/json" \
  -b cookies.txt \
  -d '{
    "link_ids": ["uuid-1", "uuid-2"],
    "category_ids": ["cat-uuid"],
    "replace": true
  }'
```

---

### Bulk Update Tags

Assign tags to multiple links at once.

**Endpoint:** `POST /api/links/bulk/tags`

**Authentication:** Required

**Request Body:**

```json
{
  "link_ids": ["uuid-1", "uuid-2"],
  "tag_ids": ["tag-uuid-1", "tag-uuid-2"],
  "replace": false
}
```

**Response:** 200 OK

```json
{
  "updated": 2
}
```

**Example:**

```bash
curl -X POST http://localhost:8080/api/links/bulk/tags \
  -H "Content-Type: application/json" \
  -b cookies.txt \
  -d '{
    "link_ids": ["uuid-1"],
    "tag_ids": ["tag-uuid"],
    "replace": false
  }'
```

---

### Link Categories Management

#### Get Link Categories

**Endpoint:** `GET /api/links/:id/categories`

Returns all categories assigned to a link.

#### Add Category to Link

**Endpoint:** `POST /api/links/:id/categories`

**Request Body:**

```json
{
  "category_id": "uuid"
}
```

#### Remove Category from Link

**Endpoint:** `DELETE /api/links/:id/categories/:category_id`

---

### Link Tags Management

#### Get Link Tags

**Endpoint:** `GET /api/links/:id/tags`

#### Add Tag to Link

**Endpoint:** `POST /api/links/:id/tags`

**Request Body:**

```json
{
  "tag_id": "uuid"
}
```

#### Remove Tag from Link

**Endpoint:** `DELETE /api/links/:id/tags/:tag_id`

---

### Link Languages Management

#### Get Link Languages

**Endpoint:** `GET /api/links/:id/languages`

#### Add Language to Link

**Endpoint:** `POST /api/links/:id/languages`

**Request Body:**

```json
{
  "language_id": "uuid"
}
```

#### Remove Language from Link

**Endpoint:** `DELETE /api/links/:id/languages/:language_id`

---

### Link Licenses Management

#### Get Link Licenses

**Endpoint:** `GET /api/links/:id/licenses`

#### Add License to Link

**Endpoint:** `POST /api/links/:id/licenses`

**Request Body:**

```json
{
  "license_id": "uuid"
}
```

#### Remove License from Link

**Endpoint:** `DELETE /api/links/:id/licenses/:license_id`

---

## Categories Endpoints

### Create Category

Create a new category.

**Endpoint:** `POST /api/categories`

**Authentication:** Required

**Request Body:**

```json
{
  "name": "Development",
  "parent_id": null,
  "level": 1
}
```

**Fields:**
- `name` - Category name (required)
- `parent_id` - Parent category UUID (optional, null for root)
- `level` - Hierarchy level: 1, 2, or 3 (required)

**Response:** 201 Created

```json
{
  "id": "uuid",
  "name": "Development",
  "user_id": "user-uuid",
  "parent_id": null,
  "level": 1,
  "created_at": "2024-01-15T10:30:00Z"
}
```

**Example:**

```bash
curl -X POST http://localhost:8080/api/categories \
  -H "Content-Type: application/json" \
  -b cookies.txt \
  -d '{
    "name": "Development",
    "parent_id": null,
    "level": 1
  }'
```

---

### List Categories

Get all categories (flat list).

**Endpoint:** `GET /api/categories`

**Authentication:** Required

**Response:** 200 OK

```json
[
  {
    "id": "uuid-1",
    "name": "Development",
    "user_id": "user-uuid",
    "parent_id": null,
    "level": 1,
    "created_at": "2024-01-15T10:30:00Z"
  },
  {
    "id": "uuid-2",
    "name": "Rust",
    "user_id": "user-uuid",
    "parent_id": "uuid-1",
    "level": 2,
    "created_at": "2024-01-15T10:30:00Z"
  }
]
```

**Example:**

```bash
curl http://localhost:8080/api/categories \
  -b cookies.txt
```

---

### Get Category Tree

Get categories as hierarchical tree structure.

**Endpoint:** `GET /api/categories/tree`

**Authentication:** Required

**Response:** 200 OK

```json
[
  {
    "id": "uuid-1",
    "name": "Development",
    "level": 1,
    "children": [
      {
        "id": "uuid-2",
        "name": "Rust",
        "level": 2,
        "children": [
          {
            "id": "uuid-3",
            "name": "Web Frameworks",
            "level": 3,
            "children": []
          }
        ]
      }
    ]
  }
]
```

**Example:**

```bash
curl http://localhost:8080/api/categories/tree \
  -b cookies.txt
```

---

### Get Category

Get a specific category by ID.

**Endpoint:** `GET /api/categories/:id`

**Authentication:** Required

**Response:** 200 OK

Returns single category object.

**Errors:**
- 404 Not Found - Category not found

---

### Update Category

Update a category's name.

**Endpoint:** `PUT /api/categories/:id`

**Authentication:** Required

**Request Body:**

```json
{
  "name": "Updated Name"
}
```

**Response:** 200 OK

Returns updated category object.

**Example:**

```bash
curl -X PUT http://localhost:8080/api/categories/uuid \
  -H "Content-Type: application/json" \
  -b cookies.txt \
  -d '{
    "name": "New Category Name"
  }'
```

---

### Delete Category

Delete a category.

**Endpoint:** `DELETE /api/categories/:id`

**Authentication:** Required

**Response:** 204 No Content

**Note:** Deleting a category removes it from all associated links.

**Example:**

```bash
curl -X DELETE http://localhost:8080/api/categories/uuid \
  -b cookies.txt
```

---

## Tags Endpoints

### Create Tag

**Endpoint:** `POST /api/tags`

**Request Body:**

```json
{
  "name": "web"
}
```

**Response:** 201 Created

```json
{
  "id": "uuid",
  "name": "web",
  "user_id": "user-uuid",
  "created_at": "2024-01-15T10:30:00Z"
}
```

---

### List Tags

**Endpoint:** `GET /api/tags`

**Response:** 200 OK

Returns array of tag objects.

---

### Delete Tag

**Endpoint:** `DELETE /api/tags/:id`

**Response:** 204 No Content

---

## Languages Endpoints

### Create Language

**Endpoint:** `POST /api/languages`

**Request Body:**

```json
{
  "name": "Rust"
}
```

**Response:** 201 Created

---

### List Languages

**Endpoint:** `GET /api/languages`

**Response:** 200 OK

Returns array of available languages (both global and user-created).

---

### Delete Language

**Endpoint:** `DELETE /api/languages/:id`

**Response:** 204 No Content

**Note:** Cannot delete global languages, only user-created ones.

---

## Licenses Endpoints

### Create License

**Endpoint:** `POST /api/licenses`

**Request Body:**

```json
{
  "name": "MIT"
}
```

**Response:** 201 Created

---

### List Licenses

**Endpoint:** `GET /api/licenses`

**Response:** 200 OK

Returns array of available licenses (both global and user-created).

---

### Delete License

**Endpoint:** `DELETE /api/licenses/:id`

**Response:** 204 No Content

**Note:** Cannot delete global licenses, only user-created ones.

---

## Scraping Endpoints

### Scrape URL

Manually scrape metadata from a URL without creating a link.

**Endpoint:** `POST /api/scrape`

**Authentication:** Required

**Request Body:**

```json
{
  "url": "https://example.com"
}
```

**Response:** 200 OK

```json
{
  "title": "Example Domain",
  "description": "Example website description",
  "favicon": "https://example.com/favicon.ico"
}
```

**Example:**

```bash
curl -X POST http://localhost:8080/api/scrape \
  -H "Content-Type: application/json" \
  -b cookies.txt \
  -d '{
    "url": "https://rust-lang.org"
  }'
```

---

## Health Endpoints

### General Health Check

**Endpoint:** `GET /api/health`

**Authentication:** None required

**Response:** 200 OK

```json
{
  "status": "healthy",
  "version": "0.1.0"
}
```

**Example:**

```bash
curl http://localhost:8080/api/health
```

---

### Database Health Check

**Endpoint:** `GET /api/health/database`

**Authentication:** None required

**Response:** 200 OK (database connected)

```json
{
  "status": "healthy",
  "connected": true
}
```

**Response:** 503 Service Unavailable (database disconnected)

```json
{
  "status": "unhealthy",
  "connected": false
}
```

**Example:**

```bash
curl http://localhost:8080/api/health/database
```

---

### Scheduler Health Check

**Endpoint:** `GET /api/health/scheduler`

**Authentication:** None required

**Response:** 200 OK

```json
{
  "status": "healthy",
  "running": true
}
```

**Example:**

```bash
curl http://localhost:8080/api/health/scheduler
```

---

## Complete Examples

### Setup and Login Flow

```bash
# 1. Check if setup is needed
curl http://localhost:8080/api/auth/check-setup

# Response: {"setup_required": true}

# 2. Create first user
curl -X POST http://localhost:8080/api/auth/setup \
  -H "Content-Type: application/json" \
  -c cookies.txt \
  -d '{
    "email": "admin@example.com",
    "password": "SecurePassword123!"
  }'

# 3. Verify login worked (session cookie saved)
curl http://localhost:8080/api/auth/me \
  -b cookies.txt

# Response: User object with your email
```

---

### Creating and Managing Links

```bash
# 1. Create a category
CATEGORY=$(curl -X POST http://localhost:8080/api/categories \
  -H "Content-Type: application/json" \
  -b cookies.txt \
  -d '{
    "name": "Rust Projects",
    "parent_id": null,
    "level": 1
  }' | jq -r '.id')

# 2. Create a tag
TAG=$(curl -X POST http://localhost:8080/api/tags \
  -H "Content-Type: application/json" \
  -b cookies.txt \
  -d '{
    "name": "web"
  }' | jq -r '.id')

# 3. Create a link with metadata
LINK=$(curl -X POST http://localhost:8080/api/links \
  -H "Content-Type: application/json" \
  -b cookies.txt \
  -d "{
    \"url\": \"https://github.com/tokio-rs/axum\",
    \"category_ids\": [\"$CATEGORY\"],
    \"tag_ids\": [\"$TAG\"]
  }" | jq -r '.id')

# 4. Get the link with all metadata
curl http://localhost:8080/api/links \
  -b cookies.txt | jq

# 5. Refresh GitHub metadata
curl -X POST "http://localhost:8080/api/links/$LINK/refresh-github" \
  -b cookies.txt

# 6. Update link title
curl -X PUT "http://localhost:8080/api/links/$LINK" \
  -H "Content-Type: application/json" \
  -b cookies.txt \
  -d '{
    "title": "Axum Web Framework"
  }'
```

---

### Search and Filter

```bash
# Search by keyword
curl "http://localhost:8080/api/links?search=rust" \
  -b cookies.txt

# Filter by category
curl "http://localhost:8080/api/links?category_id=$CATEGORY" \
  -b cookies.txt

# Combined search with sorting
curl "http://localhost:8080/api/links?search=web&tag_id=$TAG&sort=title&order=asc&limit=10" \
  -b cookies.txt

# Pagination
curl "http://localhost:8080/api/links?limit=20&offset=40" \
  -b cookies.txt
```

---

### Bulk Operations

```bash
# Bulk delete links
curl -X POST http://localhost:8080/api/links/bulk/delete \
  -H "Content-Type: application/json" \
  -b cookies.txt \
  -d '{
    "link_ids": ["uuid-1", "uuid-2", "uuid-3"]
  }'

# Bulk assign categories
curl -X POST http://localhost:8080/api/links/bulk/categories \
  -H "Content-Type: application/json" \
  -b cookies.txt \
  -d "{
    \"link_ids\": [\"uuid-1\", \"uuid-2\"],
    \"category_ids\": [\"$CATEGORY\"],
    \"replace\": false
  }"
```

---

### Export and Import

```bash
# Export all links
curl http://localhost:8080/api/links/export \
  -b cookies.txt \
  -o links_backup_$(date +%Y%m%d).json

# Import links
curl -X POST http://localhost:8080/api/links/import \
  -H "Content-Type: application/json" \
  -b cookies.txt \
  -d @links_backup.json
```

---

### Logout

```bash
# End session
curl -X POST http://localhost:8080/api/auth/logout \
  -b cookies.txt \
  -c cookies.txt

# Verify logout (should fail with 401)
curl http://localhost:8080/api/auth/me \
  -b cookies.txt
```

---

## Data Models

### Link Object

```json
{
  "id": "uuid",
  "url": "string",
  "title": "string | null",
  "description": "string | null",
  "logo": "string | null",
  "user_id": "uuid",
  "created_at": "datetime",
  "updated_at": "datetime",
  "last_metadata_update": "datetime | null",
  "github_stars": "integer | null",
  "github_language": "string | null"
}
```

### Category Object

```json
{
  "id": "uuid",
  "name": "string",
  "user_id": "uuid",
  "parent_id": "uuid | null",
  "level": "integer (1-3)",
  "created_at": "datetime"
}
```

### Tag Object

```json
{
  "id": "uuid",
  "name": "string",
  "user_id": "uuid",
  "created_at": "datetime"
}
```

### Language Object

```json
{
  "id": "uuid",
  "name": "string",
  "user_id": "uuid | null",
  "created_at": "datetime"
}
```

### License Object

```json
{
  "id": "uuid",
  "name": "string",
  "user_id": "uuid | null",
  "created_at": "datetime"
}
```

---

## Error Handling

All endpoints may return the following errors:

### Authentication Errors

```json
{
  "error": "Session expired or invalid"
}
```

HTTP Status: 401 Unauthorized

### Validation Errors

```json
{
  "error": "Invalid URL format"
}
```

HTTP Status: 400 Bad Request

### Not Found Errors

```json
{
  "error": "Link not found"
}
```

HTTP Status: 404 Not Found

### Permission Errors

```json
{
  "error": "You do not have permission to access this resource"
}
```

HTTP Status: 403 Forbidden

---

## Best Practices

1. **Session Management**
   - Save session cookie after login
   - Include cookie in all authenticated requests
   - Handle 401 errors by redirecting to login

2. **Error Handling**
   - Check HTTP status codes
   - Parse error messages from response body
   - Implement retry logic for 5XX errors

3. **Rate Limiting**
   - Respect GitHub API rate limits
   - Implement exponential backoff for failures
   - Use GitHub token for higher rate limits

4. **Pagination**
   - Use `limit` and `offset` for large datasets
   - Start with reasonable page sizes (20-50 items)
   - Implement infinite scroll or page navigation

5. **Bulk Operations**
   - Use bulk endpoints for multiple updates
   - Process in batches to avoid timeouts
   - Handle partial failures gracefully

6. **Data Consistency**
   - Refresh metadata periodically
   - Use export for backups
   - Validate data before import

---

## Version History

- **v1.0** - Initial API release
  - Authentication endpoints
  - Link management
  - Category, tag, language, license management
  - Bulk operations
  - Import/export functionality
  - Health checks

---

## Support

For API issues or questions:
- GitHub Issues: Report bugs and request features
- Documentation: Check README.md and other docs
- Source Code: Review API implementation in `src/api/`
