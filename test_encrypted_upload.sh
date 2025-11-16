#!/bin/bash

set -e

echo "🔒 TESTING ENCRYPTED FILE UPLOAD"
echo "================================="
echo ""

# Login and save cookie
echo "Step 1: Login..."
curl -s -c /tmp/cookies.txt -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"test@encrypted.com","password":"testpass123"}' > /dev/null

echo "✅ Logged in"
echo ""

# Upload file
echo "Step 2: Upload pharmaceutical data file (will be encrypted)..."
RESPONSE=$(curl -s -b /tmp/cookies.txt -X POST http://localhost:8080/api/ai-import/upload \
  -F "file=@test_pharma_data.csv")

echo "$RESPONSE" | jq '.'
echo ""

# Extract session ID
SESSION_ID=$(echo "$RESPONSE" | jq -r '.session_id')

if [ "$SESSION_ID" != "null" ] && [ -n "$SESSION_ID" ]; then
  echo "✅ File uploaded and encrypted! Session ID: $SESSION_ID"
  echo ""

  # Check if encrypted file exists
  echo "Step 3: Verifying encrypted file on disk..."
  ENCRYPTED_FILE=$(find uploads -name "*.enc" -type f -newermt "1 minute ago" | head -1)

  if [ -n "$ENCRYPTED_FILE" ]; then
    echo "✅ Found encrypted file: $ENCRYPTED_FILE"
    echo ""
    echo "File size: $(stat -c%s "$ENCRYPTED_FILE") bytes"
    echo "First 100 bytes (should be encrypted gibberish):"
    xxd -l 100 "$ENCRYPTED_FILE" || hexdump -C -n 100 "$ENCRYPTED_FILE"
    echo ""
    echo "🔒 File is encrypted on disk!"
  else
    echo "⚠️  No encrypted file found in uploads directory"
  fi
else
  echo "❌ Upload failed!"
fi

echo ""
echo "Test complete!"
