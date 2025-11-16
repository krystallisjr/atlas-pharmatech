# 🌐 Access URLs for Atlas Pharma

## Frontend
**From Windows Browser**: http://172.28.219.149:3003
**From WSL**: http://localhost:3003

## Backend API
**From Windows**: https://172.28.219.149:8443
**From WSL**: https://localhost:8443

## Login Credentials
- Email: `test@encrypted.com`
- Password: `test123`

## Important Notes

### Browser Security Issue
When you try to login, you may get a network error because:
1. Frontend runs on HTTP (http://172.28.219.149:3003)
2. Backend runs on HTTPS with self-signed cert (https://172.28.219.149:8443)
3. Browsers block "mixed content" (HTTP page calling HTTPS API with self-signed cert)

### Fix: Accept the Backend Certificate First

**Before logging in, do this:**
1. Open a new browser tab
2. Navigate to: `https://172.28.219.149:8443/api/regulatory/knowledge-base/stats`
3. You'll see a security warning about self-signed certificate
4. Click "Advanced" → "Accept the Risk and Continue" (or similar)
5. You should see JSON response with regulatory stats
6. Now go back to the login page and try again - it will work!

This tells your browser to accept the self-signed certificate from the backend.

### Alternative: Update CORS for WSL IP

The backend CORS is currently configured for localhost only. We need to add the WSL IP.

## Production Deployment

For production:
- Replace self-signed certificates with real ones (Let's Encrypt)
- Use HTTPS for both frontend and backend
- No certificate warnings
- No mixed content issues
