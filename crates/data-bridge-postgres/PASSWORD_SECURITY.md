# PostgreSQL Password Security Guide

## Overview

This document provides security guidance for handling PostgreSQL passwords when using the data-bridge PostgreSQL connector.

## Security Risks

### 1. Hardcoded Passwords

**Risk**: Passwords embedded in source code can be:
- Committed to version control (Git history)
- Exposed in exception tracebacks
- Leaked through log files
- Visible in process memory dumps
- Accidentally shared in code snippets

### 2. Plain Text Password Handling

**Risk**: Python string immutability means passwords remain in memory until garbage collected:
- Cannot be securely overwritten in Python (unlike languages with mutable byte arrays)
- May persist in memory longer than expected
- Could be exposed in core dumps or memory inspection tools

### 3. Connection String Logging

**Risk**: Connection strings containing passwords may be:
- Logged by application code
- Captured in error messages
- Displayed in debugging output

## Best Practices

### 1. Use Environment Variables (Recommended)

Store credentials in environment variables and load them at runtime:

```python
import os
from data_bridge.postgres import connection

# Recommended: Use DATABASE_URL environment variable
await connection.init(os.environ.get("DATABASE_URL"))

# Alternative: Individual parameters from environment
await connection.init(
    host=os.environ.get("PG_HOST", "localhost"),
    port=int(os.environ.get("PG_PORT", "5432")),
    database=os.environ.get("PG_DATABASE"),
    username=os.environ.get("PG_USER"),
    password=os.environ.get("PG_PASSWORD"),
)
```

**Setup environment variables:**

```bash
# Linux/macOS
export DATABASE_URL="postgres://user:password@localhost:5432/mydb"

# Or use .env file (never commit this file!)
echo "DATABASE_URL=postgres://user:password@localhost:5432/mydb" > .env
```

### 2. Use Secret Management Systems

For production environments, use dedicated secret management:

**AWS Secrets Manager:**
```python
import boto3
import json

def get_db_credentials():
    client = boto3.client('secretsmanager')
    response = client.get_secret_value(SecretId='my-db-credentials')
    return json.loads(response['SecretString'])

creds = get_db_credentials()
await connection.init(
    host=creds['host'],
    database=creds['database'],
    username=creds['username'],
    password=creds['password'],
)
```

**HashiCorp Vault:**
```python
import hvac

client = hvac.Client(url='http://vault:8200')
secret = client.secrets.kv.v2.read_secret_version(path='database/postgres')
creds = secret['data']['data']

await connection.init(
    host=creds['host'],
    database=creds['database'],
    username=creds['username'],
    password=creds['password'],
)
```

### 3. Use Configuration Files (With Proper Permissions)

If using configuration files:

1. **Never commit config files with passwords**
2. **Set restrictive file permissions**: `chmod 600 config.ini`
3. **Add to .gitignore**: `echo "config.ini" >> .gitignore`

```python
import configparser

config = configparser.ConfigParser()
config.read('config.ini')  # File mode 0600, excluded from Git

await connection.init(
    host=config['database']['host'],
    database=config['database']['name'],
    username=config['database']['user'],
    password=config['database']['password'],
)
```

**config.ini example:**
```ini
[database]
host = localhost
port = 5432
name = mydb
user = dbuser
password = secure_password_here
```

### 4. Use .env Files (Development Only)

For local development, use python-dotenv:

```python
from dotenv import load_dotenv
import os

load_dotenv()  # Load from .env file

await connection.init(os.environ.get("DATABASE_URL"))
```

**.env file (add to .gitignore):**
```
DATABASE_URL=postgres://user:password@localhost:5432/dev_db
```

## What NOT to Do

### DON'T: Hardcode Passwords

```python
# ❌ WRONG: Password in source code
await connection.init(
    host="prod-db.example.com",
    database="production",
    username="admin",
    password="SuperSecret123!",  # This will be committed to Git!
)
```

### DON'T: Log Connection Strings

```python
# ❌ WRONG: Password in logs
conn_str = "postgres://user:password@host/db"
print(f"Connecting to: {conn_str}")  # Password exposed in logs!
await connection.init(conn_str)
```

### DON'T: Store Passwords in Comments

```python
# ❌ WRONG: Password in code comments
# Production DB password: MyPassword123
await connection.init(os.environ.get("DATABASE_URL"))
```

## Development vs Production

### Development

For local development, use:
1. **`.env` files** (excluded from Git)
2. **Local environment variables**
3. **Test databases with simple passwords** (not shared with production)

### Production

For production environments, use:
1. **Cloud secret managers** (AWS Secrets Manager, Google Secret Manager, Azure Key Vault)
2. **HashiCorp Vault** or similar enterprise solutions
3. **Kubernetes secrets** (for containerized deployments)
4. **Environment variables** (set by deployment platform)

## Connection String Formats

### Standard Format
```
postgres://username:password@hostname:port/database
```

### URL Encoding for Special Characters

If password contains special characters, URL-encode them:

```python
from urllib.parse import quote_plus

password = "p@ssw0rd!#$"
encoded = quote_plus(password)
conn_str = f"postgres://user:{encoded}@host:5432/db"
```

### SSL/TLS Connections

Always use SSL for production:
```
postgres://user:pass@host:5432/db?sslmode=require
```

## Security Checklist

- [ ] Passwords stored in environment variables or secret manager
- [ ] No passwords in source code or comments
- [ ] Configuration files with passwords excluded from Git (.gitignore)
- [ ] Configuration files have restrictive permissions (chmod 600)
- [ ] SSL/TLS enabled for production database connections
- [ ] Connection strings not logged or printed
- [ ] Separate credentials for development/staging/production
- [ ] Regular password rotation policy in place
- [ ] Database user has minimum required privileges (principle of least privilege)

## Additional Resources

- [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
- [PostgreSQL SSL Documentation](https://www.postgresql.org/docs/current/ssl-tcp.html)
- [Twelve-Factor App: Config](https://12factor.net/config)
- [AWS Secrets Manager](https://aws.amazon.com/secrets-manager/)
- [HashiCorp Vault](https://www.vaultproject.io/)

## Support

For security issues or questions, please:
1. **DO NOT** create public GitHub issues containing credentials
2. Review this documentation first
3. Contact the security team privately if you suspect a credential leak
