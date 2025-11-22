#!/bin/bash

#######################################################################
# Rusty Links - Integration Test Script
#######################################################################
#
# This script runs integration tests against a running Rusty Links
# instance to verify all critical functionality works correctly.
#
# Usage:
#   ./scripts/integration-test.sh [BASE_URL]
#
# Example:
#   ./scripts/integration-test.sh http://localhost:8080
#   ./scripts/integration-test.sh https://links.yourdomain.com
#
# Requirements:
#   - curl (for HTTP requests)
#   - jq (for JSON parsing)
#   - Running Rusty Links instance
#
#######################################################################

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BASE_URL="${1:-http://localhost:8080}"
TEST_EMAIL="test-$(date +%s)@example.com"
TEST_PASSWORD="TestPassword123!"
COOKIE_FILE="/tmp/rustylinks-test-cookies.txt"
VERBOSE="${VERBOSE:-0}"

# Counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

#######################################################################
# Helper Functions
#######################################################################

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
    ((TESTS_PASSED++))
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
    ((TESTS_FAILED++))
}

log_warning() {
    echo -e "${YELLOW}[!]${NC} $1"
}

log_test() {
    ((TESTS_RUN++))
    echo -e "\n${YELLOW}Test $TESTS_RUN:${NC} $1"
}

check_prerequisites() {
    log_info "Checking prerequisites..."

    if ! command -v curl &> /dev/null; then
        log_error "curl is not installed. Please install curl."
        exit 1
    fi

    if ! command -v jq &> /dev/null; then
        log_error "jq is not installed. Please install jq."
        exit 1
    fi

    log_success "Prerequisites check passed"
}

cleanup() {
    log_info "Cleaning up..."
    rm -f "$COOKIE_FILE"
}

# Cleanup on exit
trap cleanup EXIT

#######################################################################
# Test Functions
#######################################################################

test_health_endpoint() {
    log_test "Health endpoint check"

    response=$(curl -s -w "\n%{http_code}" "${BASE_URL}/api/health")
    http_code=$(echo "$response" | tail -n 1)
    body=$(echo "$response" | head -n -1)

    if [ "$http_code" -eq 200 ]; then
        log_success "Health endpoint returned 200 OK"
        [ "$VERBOSE" -eq 1 ] && echo "Response: $body"
        return 0
    else
        log_error "Health endpoint returned $http_code (expected 200)"
        echo "Response: $body"
        return 1
    fi
}

test_check_setup() {
    log_test "Check setup status"

    response=$(curl -s -w "\n%{http_code}" "${BASE_URL}/api/auth/check-setup")
    http_code=$(echo "$response" | tail -n 1)
    body=$(echo "$response" | head -n -1)

    if [ "$http_code" -eq 200 ]; then
        setup_required=$(echo "$body" | jq -r '.setup_required')
        log_success "Setup status check returned 200 OK (setup_required: $setup_required)"
        [ "$VERBOSE" -eq 1 ] && echo "Response: $body"
        echo "$setup_required"
        return 0
    else
        log_error "Setup status check returned $http_code (expected 200)"
        echo "Response: $body"
        echo "unknown"
        return 1
    fi
}

test_create_user() {
    log_test "Create initial user via setup endpoint"

    response=$(curl -s -w "\n%{http_code}" \
        -X POST \
        -H "Content-Type: application/json" \
        -d "{\"email\":\"$TEST_EMAIL\",\"password\":\"$TEST_PASSWORD\"}" \
        "${BASE_URL}/api/auth/setup")

    http_code=$(echo "$response" | tail -n 1)
    body=$(echo "$response" | head -n -1)

    # Accept both 201 (created) and 400 (already exists)
    if [ "$http_code" -eq 201 ]; then
        log_success "User created successfully"
        [ "$VERBOSE" -eq 1 ] && echo "Response: $body"
        return 0
    elif [ "$http_code" -eq 400 ]; then
        log_warning "User already exists or setup disabled (this is okay if re-running tests)"
        [ "$VERBOSE" -eq 1 ] && echo "Response: $body"
        return 0
    else
        log_error "User creation returned unexpected status $http_code"
        echo "Response: $body"
        return 1
    fi
}

test_login() {
    log_test "Login with credentials"

    response=$(curl -s -w "\n%{http_code}" \
        -X POST \
        -H "Content-Type: application/json" \
        -c "$COOKIE_FILE" \
        -d "{\"email\":\"$TEST_EMAIL\",\"password\":\"$TEST_PASSWORD\"}" \
        "${BASE_URL}/api/auth/login")

    http_code=$(echo "$response" | tail -n 1)
    body=$(echo "$response" | head -n -1)

    if [ "$http_code" -eq 200 ]; then
        log_success "Login successful"
        [ "$VERBOSE" -eq 1 ] && echo "Response: $body"
        [ "$VERBOSE" -eq 1 ] && echo "Cookies saved to: $COOKIE_FILE"
        return 0
    else
        log_error "Login failed with status $http_code"
        echo "Response: $body"
        return 1
    fi
}

test_get_current_user() {
    log_test "Get current user info"

    response=$(curl -s -w "\n%{http_code}" \
        -b "$COOKIE_FILE" \
        "${BASE_URL}/api/auth/me")

    http_code=$(echo "$response" | tail -n 1)
    body=$(echo "$response" | head -n -1)

    if [ "$http_code" -eq 200 ]; then
        email=$(echo "$body" | jq -r '.email')
        log_success "Current user retrieved: $email"
        [ "$VERBOSE" -eq 1 ] && echo "Response: $body"
        return 0
    else
        log_error "Get current user failed with status $http_code"
        echo "Response: $body"
        return 1
    fi
}

test_create_link() {
    log_test "Create a new link"

    local url="https://github.com/rust-lang/rust"

    response=$(curl -s -w "\n%{http_code}" \
        -X POST \
        -H "Content-Type: application/json" \
        -b "$COOKIE_FILE" \
        -d "{\"url\":\"$url\",\"title\":\"Rust Programming Language\"}" \
        "${BASE_URL}/api/links")

    http_code=$(echo "$response" | tail -n 1)
    body=$(echo "$response" | head -n -1)

    # Accept both 201 (created) and 409 (duplicate)
    if [ "$http_code" -eq 201 ]; then
        link_id=$(echo "$body" | jq -r '.id')
        log_success "Link created successfully (ID: $link_id)"
        [ "$VERBOSE" -eq 1 ] && echo "Response: $body"
        echo "$link_id"
        return 0
    elif [ "$http_code" -eq 409 ]; then
        log_warning "Link already exists (this is okay if re-running tests)"
        [ "$VERBOSE" -eq 1 ] && echo "Response: $body"
        # Try to get the existing link
        test_list_links > /dev/null
        return 0
    else
        log_error "Create link failed with status $http_code"
        echo "Response: $body"
        echo ""
        return 1
    fi
}

test_list_links() {
    log_test "List all links"

    response=$(curl -s -w "\n%{http_code}" \
        -b "$COOKIE_FILE" \
        "${BASE_URL}/api/links")

    http_code=$(echo "$response" | tail -n 1)
    body=$(echo "$response" | head -n -1)

    if [ "$http_code" -eq 200 ]; then
        total=$(echo "$body" | jq -r '.total')
        log_success "Links retrieved successfully (total: $total)"
        [ "$VERBOSE" -eq 1 ] && echo "Response: $body"
        return 0
    else
        log_error "List links failed with status $http_code"
        echo "Response: $body"
        return 1
    fi
}

test_search_links() {
    log_test "Search links by query"

    response=$(curl -s -w "\n%{http_code}" \
        -b "$COOKIE_FILE" \
        "${BASE_URL}/api/links?q=rust")

    http_code=$(echo "$response" | tail -n 1)
    body=$(echo "$response" | head -n -1)

    if [ "$http_code" -eq 200 ]; then
        total=$(echo "$body" | jq -r '.total')
        log_success "Search completed successfully (results: $total)"
        [ "$VERBOSE" -eq 1 ] && echo "Response: $body"
        return 0
    else
        log_error "Search failed with status $http_code"
        echo "Response: $body"
        return 1
    fi
}

test_create_category() {
    log_test "Create a category"

    response=$(curl -s -w "\n%{http_code}" \
        -X POST \
        -H "Content-Type: application/json" \
        -b "$COOKIE_FILE" \
        -d "{\"name\":\"Test Category\",\"description\":\"Test category for integration tests\"}" \
        "${BASE_URL}/api/categories")

    http_code=$(echo "$response" | tail -n 1)
    body=$(echo "$response" | head -n -1)

    # Accept both 201 (created) and 409 (duplicate)
    if [ "$http_code" -eq 201 ]; then
        category_id=$(echo "$body" | jq -r '.id')
        log_success "Category created successfully (ID: $category_id)"
        [ "$VERBOSE" -eq 1 ] && echo "Response: $body"
        echo "$category_id"
        return 0
    elif [ "$http_code" -eq 409 ]; then
        log_warning "Category already exists (this is okay if re-running tests)"
        [ "$VERBOSE" -eq 1 ] && echo "Response: $body"
        echo ""
        return 0
    else
        log_error "Create category failed with status $http_code"
        echo "Response: $body"
        echo ""
        return 1
    fi
}

test_list_categories() {
    log_test "List all categories"

    response=$(curl -s -w "\n%{http_code}" \
        -b "$COOKIE_FILE" \
        "${BASE_URL}/api/categories")

    http_code=$(echo "$response" | tail -n 1)
    body=$(echo "$response" | head -n -1)

    if [ "$http_code" -eq 200 ]; then
        count=$(echo "$body" | jq '. | length')
        log_success "Categories retrieved successfully (count: $count)"
        [ "$VERBOSE" -eq 1 ] && echo "Response: $body"
        return 0
    else
        log_error "List categories failed with status $http_code"
        echo "Response: $body"
        return 1
    fi
}

test_create_tag() {
    log_test "Create a tag"

    response=$(curl -s -w "\n%{http_code}" \
        -X POST \
        -H "Content-Type: application/json" \
        -b "$COOKIE_FILE" \
        -d "{\"name\":\"test-tag\"}" \
        "${BASE_URL}/api/tags")

    http_code=$(echo "$response" | tail -n 1)
    body=$(echo "$response" | head -n -1)

    # Accept both 201 (created) and 409 (duplicate)
    if [ "$http_code" -eq 201 ]; then
        tag_id=$(echo "$body" | jq -r '.id')
        log_success "Tag created successfully (ID: $tag_id)"
        [ "$VERBOSE" -eq 1 ] && echo "Response: $body"
        echo "$tag_id"
        return 0
    elif [ "$http_code" -eq 409 ]; then
        log_warning "Tag already exists (this is okay if re-running tests)"
        [ "$VERBOSE" -eq 1 ] && echo "Response: $body"
        echo ""
        return 0
    else
        log_error "Create tag failed with status $http_code"
        echo "Response: $body"
        echo ""
        return 1
    fi
}

test_list_languages() {
    log_test "List all languages"

    response=$(curl -s -w "\n%{http_code}" \
        -b "$COOKIE_FILE" \
        "${BASE_URL}/api/languages")

    http_code=$(echo "$response" | tail -n 1)
    body=$(echo "$response" | head -n -1)

    if [ "$http_code" -eq 200 ]; then
        count=$(echo "$body" | jq '. | length')
        log_success "Languages retrieved successfully (count: $count)"
        [ "$VERBOSE" -eq 1 ] && echo "Response: $body"
        return 0
    else
        log_error "List languages failed with status $http_code"
        echo "Response: $body"
        return 1
    fi
}

test_list_licenses() {
    log_test "List all licenses"

    response=$(curl -s -w "\n%{http_code}" \
        -b "$COOKIE_FILE" \
        "${BASE_URL}/api/licenses")

    http_code=$(echo "$response" | tail -n 1)
    body=$(echo "$response" | head -n -1)

    if [ "$http_code" -eq 200 ]; then
        count=$(echo "$body" | jq '. | length')
        log_success "Licenses retrieved successfully (count: $count)"
        [ "$VERBOSE" -eq 1 ] && echo "Response: $body"
        return 0
    else
        log_error "List licenses failed with status $http_code"
        echo "Response: $body"
        return 1
    fi
}

test_unauthorized_access() {
    log_test "Verify authentication is required"

    # Try to access protected endpoint without cookies
    response=$(curl -s -w "\n%{http_code}" \
        "${BASE_URL}/api/links")

    http_code=$(echo "$response" | tail -n 1)
    body=$(echo "$response" | head -n -1)

    if [ "$http_code" -eq 401 ]; then
        log_success "Unauthorized access correctly blocked (401)"
        [ "$VERBOSE" -eq 1 ] && echo "Response: $body"
        return 0
    else
        log_error "Expected 401 Unauthorized, got $http_code"
        echo "Response: $body"
        return 1
    fi
}

test_logout() {
    log_test "Logout"

    response=$(curl -s -w "\n%{http_code}" \
        -X POST \
        -b "$COOKIE_FILE" \
        -c "$COOKIE_FILE" \
        "${BASE_URL}/api/auth/logout")

    http_code=$(echo "$response" | tail -n 1)
    body=$(echo "$response" | head -n -1)

    if [ "$http_code" -eq 200 ]; then
        log_success "Logout successful"
        [ "$VERBOSE" -eq 1 ] && echo "Response: $body"
        return 0
    else
        log_error "Logout failed with status $http_code"
        echo "Response: $body"
        return 1
    fi
}

#######################################################################
# Main Test Flow
#######################################################################

run_all_tests() {
    log_info "Starting integration tests for Rusty Links"
    log_info "Target URL: $BASE_URL"
    log_info "Test email: $TEST_EMAIL"
    echo ""

    # Phase 1: Unauthenticated endpoints
    log_info "=== Phase 1: Unauthenticated Endpoints ==="
    test_health_endpoint || true
    setup_required=$(test_check_setup) || true
    test_unauthorized_access || true
    echo ""

    # Phase 2: User setup and authentication
    log_info "=== Phase 2: Authentication ==="
    if [ "$setup_required" = "true" ] || [ "$setup_required" = "unknown" ]; then
        test_create_user || true
    else
        log_warning "Setup not required, skipping user creation"
    fi
    test_login || exit 1  # Exit if login fails, can't continue
    test_get_current_user || true
    echo ""

    # Phase 3: Link management
    log_info "=== Phase 3: Link Management ==="
    link_id=$(test_create_link) || true
    test_list_links || true
    test_search_links || true
    echo ""

    # Phase 4: Organization (categories, tags, languages, licenses)
    log_info "=== Phase 4: Organization ==="
    category_id=$(test_create_category) || true
    test_list_categories || true
    tag_id=$(test_create_tag) || true
    test_list_languages || true
    test_list_licenses || true
    echo ""

    # Phase 5: Logout
    log_info "=== Phase 5: Session Management ==="
    test_logout || true
    echo ""
}

print_summary() {
    echo ""
    echo "========================================"
    echo "           TEST SUMMARY"
    echo "========================================"
    echo -e "Total tests run:    ${BLUE}$TESTS_RUN${NC}"
    echo -e "Tests passed:       ${GREEN}$TESTS_PASSED${NC}"
    echo -e "Tests failed:       ${RED}$TESTS_FAILED${NC}"
    echo "========================================"

    if [ "$TESTS_FAILED" -eq 0 ]; then
        echo -e "${GREEN}All tests passed! ✓${NC}"
        return 0
    else
        echo -e "${RED}Some tests failed. Please review the output above.${NC}"
        return 1
    fi
}

#######################################################################
# Entry Point
#######################################################################

main() {
    check_prerequisites
    run_all_tests
    print_summary
}

# Run main function
main
exit_code=$?

# Exit with appropriate code
exit $exit_code
