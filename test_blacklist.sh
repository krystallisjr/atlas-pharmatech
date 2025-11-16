#!/bin/bash

echo "TEST 1: Login and get token"
TOKEN=$(curl -s -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"production@secure.com","password":"SecurePass123!"}' \
  | jq -r '.token')
echo "Token: ${TOKEN:0:50}..."
echo ""

echo "TEST 2: Use token to access protected endpoint (should work)"
curl -s -X GET http://localhost:8080/api/auth/profile \
  -H "Authorization: Bearer $TOKEN" \
  | jq '.'
echo ""

echo "TEST 3: Logout (blacklist the token)"
curl -s -X POST http://localhost:8080/api/auth/logout \
  -H "Authorization: Bearer $TOKEN"
echo ""
echo ""

echo "TEST 4: Try to use the same token again (should FAIL with 401)"
HTTP_CODE=$(curl -s -w "%{http_code}" -o /tmp/response.json -X GET http://localhost:8080/api/auth/profile \
  -H "Authorization: Bearer $TOKEN")
echo "HTTP Status: $HTTP_CODE"
cat /tmp/response.json
echo ""

if [ "$HTTP_CODE" = "401" ]; then
  echo "SUCCESS! Token was properly blacklisted after logout"
else
  echo "FAILURE! Token still works after logout (HTTP $HTTP_CODE)"
fi
