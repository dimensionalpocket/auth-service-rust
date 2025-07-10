# API Documentation

## REST Endpoints

### GET /
Returns service status.

**Response:**
```
OK
```

### GET /health
Returns health check status.

**Response:**
```
OK
```

## GraphQL Endpoint

### POST /graphql

**Query: getServerTimestamp**
```graphql
query {
  getServerTimestamp
}
```

**Response:**
```json
{
  "data": {
    "getServerTimestamp": "1706356800000"
  }
}
```

## Usage Examples

### REST Endpoints
```bash
# Test root endpoint
curl http://localhost:3000/

# Test health endpoint
curl http://localhost:3000/health
```

### GraphQL Query
```bash
# Test GraphQL endpoint
curl -X POST http://localhost:3000/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ getServerTimestamp }"}'
```