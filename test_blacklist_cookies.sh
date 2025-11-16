#!/bin/bash

echo "Test Token Blacklist with Cookies"
echo "=================================="
echo ""

echo "Step 1: Login and save cookie"
curl -s -c /tmp/cookies.txt -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"test@encrypted.com","password":"testpass123"}' | jq '.email'

echo ""
echo "Step 2: Access profile WITH cookie (should work)"
curl -s -b /tmp/cookies.txt -X GET http://localhost:8080/api/auth/profile | jq '.email'

echo ""
echo "Step 3: Logout (blacklists token in cookie)"
curl -s -b /tmp/cookies.txt -X POST http://localhost:8080/api/auth/logout
echo "Logged out"

echo ""
echo "Step 4: Try to access profile with same cookie (should FAIL - 401)"
HTTP_CODE=$(curl -s -b /tmp/cookies.txt -w "%{http_code}" -o /dev/null -X GET http://localhost:8080/api/auth/profile)
echo "HTTP Status: $HTTP_CODE"

if [ "$HTTP_CODE" = "401" ]; then
  echo "SUCCESS! Token blacklist is working!"
else
  echo "FAIL! Token still works (HTTP $HTTP_CODE)"
fi
