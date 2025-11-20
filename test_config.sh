#!/bin/bash
# Test script to demonstrate configuration validation
# This script would normally be run with: cargo run

echo "=== Configuration Testing Scenarios ==="
echo ""

echo "Test 1: Valid configuration (.env file present)"
echo "Expected: Application starts successfully, logs masked database URL"
echo ""

echo "Test 2: Missing DATABASE_URL"
echo "To test: Remove DATABASE_URL from .env"
echo "Expected: Error - 'Missing required environment variable: DATABASE_URL'"
echo ""

echo "Test 3: Invalid APP_PORT (non-numeric)"
echo "To test: Set APP_PORT=invalid in .env"
echo "Expected: Error - 'Failed to parse APP_PORT: invalid digit found in string'"
echo ""

echo "Test 4: Invalid UPDATE_INTERVAL_DAYS (< 1)"
echo "To test: Set UPDATE_INTERVAL_DAYS=0 in .env"
echo "Expected: Error - 'Invalid value for UPDATE_INTERVAL_DAYS: must be at least 1'"
echo ""

echo "Test 5: Optional UPDATE_INTERVAL_DAYS not set"
echo "To test: Remove UPDATE_INTERVAL_DAYS from .env"
echo "Expected: Uses default value of 30"
echo ""

echo "Test 6: Database URL masking"
echo "Expected: Password is masked in logs (postgresql://rustylinks:****@localhost:5432/rusty_links)"
echo ""

echo "=== To run tests ==="
echo "1. Ensure .env file is configured"
echo "2. Run: cargo run"
echo "3. Check the JSON log output for configuration details"
echo ""
