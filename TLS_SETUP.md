# 🔒 TLS/HTTPS Configuration Guide

## Production-Ready Security for Atlas Pharma

This guide covers TLS/HTTPS setup for both **development** and **production** environments.

---

## 📋 Table of Contents

1. [Why TLS is Critical](#why-tls-is-critical)
2. [Development Setup (Self-Signed Certificates)](#development-setup)
3. [Production Setup (Let's Encrypt)](#production-setup)
4. [Security Best Practices](#security-best-practices)
5. [Troubleshooting](#troubleshooting)

---

## 🔐 Why TLS is Critical

**TLS/HTTPS is MANDATORY for production** because:

✅ **Protects MFA/TOTP codes** - Without TLS, TOTP codes are transmitted in plaintext and can be intercepted
✅ **Secures authentication tokens** - JWT tokens and session cookies must be encrypted in transit
✅ **Prevents man-in-the-middle attacks** - TLS verifies server identity
✅ **Protects PHI/PII data** - Required for HIPAA compliance
✅ **Enables secure cookies** - `Secure` flag requires HTTPS
✅ **Compliance requirement** - SOC 2, PCI-DSS, HIPAA all require TLS

**⚠️ WARNING:** Running production without TLS is a **critical security vulnerability**. All sensitive data (passwords, MFA codes, PHI) will be transmitted in plaintext.

---

## 🛠️ Development Setup (Self-Signed Certificates)

### Step 1: Generate Self-Signed Certificates

```bash
# Create certificates directory
mkdir -p certs

# Generate 4096-bit RSA certificate valid for 1 year
openssl req -x509 -newkey rsa:4096 \
  -keyout certs/key.pem \
  -out certs/cert.pem \
  -days 365 -nodes \
  -subj '/CN=localhost/O=Atlas Pharma/C=US'
```

**What this does:**
- `-x509`: Self-signed certificate
- `-newkey rsa:4096`: 4096-bit RSA key (strong encryption)
- `-days 365`: Valid for 1 year
- `-nodes`: No passphrase (for development convenience)
- `-subj '/CN=localhost'`: Certificate for localhost

### Step 2: Configure Environment Variables

Add to `.env`:

```bash
# 🔒 TLS/HTTPS Configuration
TLS_ENABLED=true
TLS_CERT_PATH=./certs/cert.pem
TLS_KEY_PATH=./certs/key.pem
TLS_PORT=8443
```

### Step 3: Update CORS Origins

Add HTTPS origins to `.env`:

```bash
CORS_ORIGINS=http://localhost:3000,http://localhost:3001,https://localhost:3000,https://localhost:3001
```

### Step 4: Start Server

```bash
cargo run --release
```

**You should see:**
```
✅ TLS configured with certificate: "./certs/cert.pem"
🔒 Starting Atlas Pharma server with TLS on https://0.0.0.0:8443
```

### Step 5: Test HTTPS Endpoint

```bash
curl -k https://localhost:8443/api/mfa/status
```

**Note:** `-k` flag bypasses certificate validation (self-signed cert). In production, this flag should NEVER be used.

### Step 6: Configure Frontend

Update frontend API base URL to use HTTPS:

```typescript
// atlas-frontend/src/lib/api-client.ts
baseURL: process.env.NEXT_PUBLIC_API_URL || 'https://localhost:8443'
```

**Browser Warning:** Self-signed certificates will trigger browser security warnings. You can:
- Click "Advanced" → "Proceed to localhost (unsafe)"
- Or add certificate to system trust store (macOS/Linux)

---

## 🚀 Production Setup (Let's Encrypt)

### Prerequisites

- Domain name pointing to your server
- Server with public IP (DigitalOcean, AWS, etc.)
- Ports 80 and 443 open in firewall

### Step 1: Install Certbot

**Ubuntu/Debian:**
```bash
sudo apt-get update
sudo apt-get install certbot
```

**RHEL/CentOS:**
```bash
sudo yum install certbot
```

**Docker:**
```bash
docker pull certbot/certbot
```

### Step 2: Generate Let's Encrypt Certificate

**Standalone mode** (recommended for first-time setup):

```bash
# Stop your server first
sudo systemctl stop atlas-pharma

# Generate certificate
sudo certbot certonly --standalone -d api.atlaspharma.com

# Restart server
sudo systemctl start atlas-pharma
```

**Webroot mode** (if you have existing web server):

```bash
sudo certbot certonly --webroot -w /var/www/html -d api.atlaspharma.com
```

**Certificate files will be at:**
- Certificate: `/etc/letsencrypt/live/api.atlaspharma.com/fullchain.pem`
- Private key: `/etc/letsencrypt/live/api.atlaspharma.com/privkey.pem`

### Step 3: Configure Production Environment

**Production `.env`:**

```bash
# 🔒 PRODUCTION TLS Configuration
TLS_ENABLED=true
TLS_CERT_PATH=/etc/letsencrypt/live/api.atlaspharma.com/fullchain.pem
TLS_KEY_PATH=/etc/letsencrypt/live/api.atlaspharma.com/privkey.pem
TLS_PORT=443

# Production CORS
CORS_ORIGINS=https://atlaspharma.com,https://www.atlaspharma.com

# Disable debug logging
RUST_LOG=info
```

### Step 4: Set File Permissions

Let's Encrypt certificates are owned by root. Grant read access:

```bash
# Option 1: Add your service user to certbot group
sudo usermod -aG certbot atlas-pharma

# Option 2: Copy certificates to application directory
sudo cp /etc/letsencrypt/live/api.atlaspharma.com/fullchain.pem /opt/atlas-pharma/certs/
sudo cp /etc/letsencrypt/live/api.atlaspharma.com/privkey.pem /opt/atlas-pharma/certs/
sudo chown atlas-pharma:atlas-pharma /opt/atlas-pharma/certs/*
sudo chmod 600 /opt/atlas-pharma/certs/privkey.pem
```

### Step 5: Configure Auto-Renewal

Let's Encrypt certificates expire every 90 days. Set up auto-renewal:

```bash
# Test renewal (dry run)
sudo certbot renew --dry-run

# Add cron job for auto-renewal
sudo crontab -e
```

**Add this line:**
```cron
# Renew Let's Encrypt certificates daily at 2 AM
0 2 * * * certbot renew --quiet --post-hook "systemctl reload atlas-pharma"
```

**Alternative with timer (systemd):**
```bash
sudo systemctl enable certbot-renew.timer
sudo systemctl start certbot-renew.timer
```

### Step 6: Configure Firewall

```bash
# Allow HTTPS traffic
sudo ufw allow 443/tcp

# Allow HTTP for certificate renewal (optional)
sudo ufw allow 80/tcp

# Verify firewall rules
sudo ufw status
```

### Step 7: Test Production HTTPS

```bash
# Test from outside server
curl https://api.atlaspharma.com/api/mfa/status

# Check certificate
openssl s_client -connect api.atlaspharma.com:443 -showcerts

# Verify with SSL Labs (highly recommended)
# Visit: https://www.ssllabs.com/ssltest/analyze.html?d=api.atlaspharma.com
```

---

## 🛡️ Security Best Practices

### TLS Configuration Hardening

The Atlas Pharma backend already implements:

✅ **TLS 1.3 and TLS 1.2 only** - No SSLv3, TLS 1.0, or TLS 1.1
✅ **Strong cipher suites** - AES-256-GCM preferred
✅ **Perfect forward secrecy** - ECDHE key exchange
✅ **Certificate validation** - Rejects invalid/expired certificates

### Cookie Security

When TLS is enabled, all cookies use:
- `Secure` flag (HTTPS only)
- `HttpOnly` flag (no JavaScript access)
- `SameSite=Strict` (CSRF protection)

### CORS Security

Only whitelisted origins in `CORS_ORIGINS` can access the API. HTTPS origins should be used in production.

### Certificate Best Practices

✅ **Use strong key sizes** - Minimum 2048-bit RSA or 256-bit ECDSA
✅ **Rotate certificates** - Before expiration (Let's Encrypt auto-renews)
✅ **Monitor expiration** - Set up alerts 30 days before expiry
✅ **Use full chain** - Include intermediate certificates
✅ **Protect private keys** - Permissions: 600 (owner read/write only)
✅ **Never commit keys** - Add `certs/` to `.gitignore`

---

## 🔧 Troubleshooting

### "Address already in use" Error

**Problem:** Port 8443 or 443 already in use

**Solution:**
```bash
# Find process using port
sudo lsof -i :8443

# Kill process
sudo kill -9 <PID>

# Or change port in .env
TLS_PORT=8444
```

### Certificate Not Found

**Problem:** `TLS certificate not found at "./certs/cert.pem"`

**Solution:**
```bash
# Verify files exist
ls -l certs/

# Check file permissions
chmod 644 certs/cert.pem
chmod 600 certs/key.pem

# Verify path in .env
cat .env | grep TLS_CERT_PATH
```

### Browser Shows "Not Secure" Warning

**Problem:** Self-signed certificate not trusted

**Solutions:**

**Development (Accept Risk):**
- Click "Advanced" → "Proceed to localhost (unsafe)"

**Development (Add to Trust Store):**

**macOS:**
```bash
sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain certs/cert.pem
```

**Ubuntu/Debian:**
```bash
sudo cp certs/cert.pem /usr/local/share/ca-certificates/atlas-pharma.crt
sudo update-ca-certificates
```

**Production:** Use Let's Encrypt (automatically trusted by all browsers)

### Let's Encrypt Rate Limits

**Problem:** "too many certificates already issued"

**Solution:**
- Let's Encrypt limits: 50 certificates per domain per week
- Use staging environment for testing: `--staging` flag
- Wait 7 days for rate limit reset
- Use DNS-01 challenge for wildcard certificates

### Connection Refused

**Problem:** Cannot connect to HTTPS endpoint

**Checklist:**
```bash
# 1. Verify server is running
ps aux | grep atlas-pharma

# 2. Check if listening on port
sudo netstat -tlnp | grep 8443

# 3. Test locally first
curl -k https://localhost:8443/api/mfa/status

# 4. Check firewall
sudo ufw status

# 5. Verify DNS resolution (production)
nslookup api.atlaspharma.com
```

---

## 📊 Certificate Verification Checklist

Before going to production, verify:

- [ ] Certificate is valid (not expired)
- [ ] Certificate matches domain name
- [ ] Full certificate chain is included
- [ ] Private key permissions are 600
- [ ] TLS 1.3 is enabled
- [ ] Strong cipher suites are used
- [ ] HTTPS redirect works (if configured)
- [ ] Auto-renewal is configured
- [ ] Monitoring/alerts are set up
- [ ] SSL Labs grade is A or A+

**Test with SSL Labs:**
https://www.ssllabs.com/ssltest/analyze.html?d=api.atlaspharma.com

**Expected Grade:** A or A+

---

## 📚 Additional Resources

- **Let's Encrypt Documentation:** https://letsencrypt.org/docs/
- **Mozilla SSL Configuration Generator:** https://ssl-config.mozilla.org/
- **SSL Labs Server Test:** https://www.ssllabs.com/ssltest/
- **Certbot Documentation:** https://certbot.eff.org/docs/
- **Rustls Documentation:** https://docs.rs/rustls/

---

## 🚨 Security Warnings

**❌ NEVER in Production:**
- Use self-signed certificates
- Disable certificate validation
- Use weak cipher suites
- Use TLS 1.0 or 1.1
- Commit private keys to git
- Use HTTP for sensitive data
- Skip certificate renewal

**✅ ALWAYS in Production:**
- Use Let's Encrypt or commercial CA
- Enable TLS 1.3
- Use strong encryption (AES-256)
- Set up auto-renewal
- Monitor certificate expiration
- Use HTTPS-only cookies
- Implement HSTS headers

---

## 📞 Support

For TLS/certificate issues:

1. Check server logs: `journalctl -u atlas-pharma -f`
2. Test with curl: `curl -v https://api.atlaspharma.com`
3. Verify with openssl: `openssl s_client -connect api.atlaspharma.com:443`
4. Check SSL Labs: https://www.ssllabs.com/ssltest/

---

**Last Updated:** 2025-11-14
**Atlas Pharma Version:** 1.0.0
**Security Level:** Production-Ready ✅
