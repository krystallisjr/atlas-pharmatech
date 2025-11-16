#!/bin/bash

echo "Step 1: Register new user"
curl -s -X POST http://localhost:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"blacklisttest@test.com","password":"TestPass123!","company_name":"Test Co","contact_person":"John Doe","phone":"555-1234","address":"123 Test St","license_number":"LIC123"}' | jq '.'

echo ""
echo "Step 2: Login to get token"
TOKEN=$(curl -s -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"blacklisttest@test.com","password":"TestPass123!"}' \
  | jq -r '.token')
echo "Token: ${TOKEN:0:60}..."

echo ""
echo "Step 3: Access protected endpoint WITH token (should work)"
curl -s -X GET http://localhost:8080/api/auth/profile \
  -H "Authorization: Bearer $TOKEN" | jq '.email'

echo ""
echo "Step 4: Logout (blacklists the token)"
curl -s -X POST http://localhost:8080/api/auth/logout \
  -H "Authorization: Bearer $TOKEN"

echo ""
echo ""
echo "Step 5: Try to use SAME token again (should FAIL with 401)"
HTTP_CODE=$(curl -s -w "%{http_code}" -o /dev/null -X GET http://localhost:8080/api/auth/profile \
  -H "Authorization: Bearer $TOKEN")
echo "HTTP Status: $HTTP_CODE"

if [ "$HTTP_CODE" = "401" ]; then
  echo "SUCCESS! Token was properly blacklisted!"
else
  echo "FAIL! Token still works after logout (HTTP $HTTP_CODE)"
fi
