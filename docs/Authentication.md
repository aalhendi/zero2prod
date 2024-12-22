# Authentication System

## Overview

The application uses Argon2id for password hashing combined with session-based authentication. A server-side pepper adds an additional security layer against database compromises.

## Implementation Details

### Password Security

- **Hashing**: Argon2id (v0x13)
  - Memory: 15000 KB
  - Iterations: 2
  - Parallelism: 1
- **Additional Security**: Server-side pepper
- **Storage**: PHC string format
- **Performance**: CPU-intensive operations run in dedicated thread pool

### Authentication Flow

1. **Session Management**

   - Typed sessions store UUID and session data
   - Middleware validates session state
   - User IDs available via request extensions

2. **Request Handling**
   - Anonymous requests redirect to login
   - Authenticated requests include user context
   - Session errors trigger clearance

## Configuration

### Development

```toml
[auth]
pepper = "your-secret-pepper-value"  # Default development value
```

### Production

Required configuration:

```toml
[auth]
pepper = "<strong-secret-value>"  # Must be changed
```

### Default Admin Account

```
Username: admin
Password: everythinghastostartsomewhere
ID: 7292210f-f3b9-4abc-b256-8b49a139c062
```

⚠️ Change credentials immediately after first login

## Security Notes

- Never use development pepper in production
- Set secure session timeouts
- Configure HTTPS
- Monitor authentication failures
