# PyLoop CRUD - Auto-Generated REST Endpoints

**Phase 3 of PyLoop Implementation**: Declarative DSL for automatically generating CRUD endpoints from Document models.

## Overview

The PyLoop CRUD feature allows you to automatically generate 5 RESTful endpoints for any Document model using a simple decorator syntax. This eliminates boilerplate code and provides a consistent API pattern across your application.

## Features

- **Auto-Generated Endpoints**: Create 5 REST endpoints with a single decorator
- **Zero Boilerplate**: No manual handler writing required
- **Pagination Support**: Built-in pagination for list endpoints
- **Error Handling**: Automatic 400/404 error responses
- **Type Safety**: Leverages Document model validation
- **Customizable**: Override prefix and tags as needed

## Quick Start

```python
from data_bridge.mongodb import Document, init_db
from data_bridge.pyloop import App
import asyncio

# Define your Document model
class Product(Document):
    name: str
    price: float
    stock: int = 0

    class Settings:
        name = "products"

# Create app
app = App(title="Product API", version="1.0.0")

# Auto-generate CRUD endpoints (direct call - RECOMMENDED)
app.crud_routes(Product)

# OR use decorator syntax (legacy, still supported)
# @app.crud(Product)
# class ProductCRUD:
#     pass

# Initialize database and run
async def main():
    await init_db(
        database="my_database",
        connection_string="mongodb://localhost:27017"
    )

asyncio.run(main())
app.serve(host="127.0.0.1", port=8000)
```

This generates 5 endpoints:

1. `GET /products?skip=0&limit=10` - List products with pagination
2. `GET /products/{id}` - Get product by ID
3. `POST /products` - Create new product
4. `PUT /products/{id}` - Update product by ID
5. `DELETE /products/{id}` - Delete product by ID

## API Reference

### `app.crud_routes(document_cls, prefix=None, tags=None, operations=None, create=True, read=True, update=True, delete=True, list=True)` (RECOMMENDED)

Auto-generate CRUD endpoints for a Document model with granular control.

**Parameters:**

- `document_cls` (Document): The Document class to generate CRUD endpoints for
- `prefix` (str, optional): URL prefix for the endpoints. Defaults to `/{collection_name}`
- `tags` (list, optional): OpenAPI tags for the endpoints. Defaults to `[collection_name]`
- `operations` (str, optional): String specifying operations (e.g., "CRUDL", "CR", "RL")
  - `C` = Create (POST)
  - `R` = Read (GET /{id})
  - `U` = Update (PUT /{id})
  - `D` = Delete (DELETE /{id})
  - `L` = List (GET / with pagination)
  - If provided, overrides individual boolean flags
- `create` (bool): Generate POST endpoint (default: True)
- `read` (bool): Generate GET /{id} endpoint (default: True)
- `update` (bool): Generate PUT /{id} endpoint (default: True)
- `delete` (bool): Generate DELETE /{id} endpoint (default: True)
- `list` (bool): Generate GET / endpoint with pagination (default: True)

**Examples:**

```python
# All operations (default)
app.crud_routes(Product)

# Read-only API (string shorthand)
app.crud_routes(Product, operations="RL")  # Only Read + List

# Create and read only (string shorthand)
app.crud_routes(Product, operations="CR")

# Explicit control with boolean flags
app.crud_routes(Product, create=True, read=True, update=False, delete=False)

# Custom prefix
app.crud_routes(Product, prefix="/api/v1/products")

# Multiple options combined
app.crud_routes(
    Product,
    prefix="/api/products",
    tags=["inventory", "products"],
    operations="RU"  # Only Read + Update
)
```

### `app.crud(document_cls, prefix=None, tags=None)` (LEGACY)

Legacy decorator-style CRUD generation. Use `crud_routes()` instead for direct method call.

**Parameters:**

- `document_cls` (Document): The Document class to generate CRUD endpoints for
- `prefix` (str, optional): URL prefix for the endpoints. Defaults to `/{collection_name}`
- `tags` (list, optional): OpenAPI tags for the endpoints. Defaults to `[collection_name]`

**Returns:**

- Decorator function that can be applied to a class

**Example:**

```python
@app.crud(Product, prefix="/api/products", tags=["inventory", "products"])
class ProductCRUD:
    pass
```

**Note:** This decorator syntax is kept for backward compatibility. New code should use `crud_routes()` for a cleaner API.

## Generated Endpoints

### 1. List Documents

**Endpoint:** `GET {prefix}?skip=0&limit=10`

**Query Parameters:**
- `skip` (int): Number of documents to skip (default: 0)
- `limit` (int): Maximum number of documents to return (default: 10, max: 100)

**Response:** `200 OK`
```json
{
  "items": [
    {"_id": "...", "name": "Laptop", "price": 999.99, "stock": 50},
    {"_id": "...", "name": "Mouse", "price": 29.99, "stock": 100}
  ],
  "skip": 0,
  "limit": 10,
  "total": 2
}
```

**Example:**
```bash
curl "http://127.0.0.1:8000/products?skip=0&limit=10"
```

### 2. Get Document by ID

**Endpoint:** `GET {prefix}/{id}`

**Path Parameters:**
- `id` (string): MongoDB ObjectId as hex string

**Response:** `200 OK`
```json
{
  "_id": "507f1f77bcf86cd799439011",
  "name": "Laptop",
  "price": 999.99,
  "stock": 50
}
```

**Error Response:** `404 Not Found`
```json
{
  "error": "Not found",
  "id": "507f1f77bcf86cd799439011"
}
```

**Example:**
```bash
curl http://127.0.0.1:8000/products/507f1f77bcf86cd799439011
```

### 3. Create Document

**Endpoint:** `POST {prefix}`

**Request Body:**
```json
{
  "name": "Laptop",
  "price": 999.99,
  "stock": 50
}
```

**Response:** `201 Created`
```json
{
  "_id": "507f1f77bcf86cd799439011",
  "name": "Laptop",
  "price": 999.99,
  "stock": 50
}
```

**Error Response:** `400 Bad Request`
```json
{
  "error": "Request body required"
}
```

**Example:**
```bash
curl -X POST http://127.0.0.1:8000/products \
  -H 'Content-Type: application/json' \
  -d '{"name": "Laptop", "price": 999.99, "stock": 50}'
```

### 4. Update Document

**Endpoint:** `PUT {prefix}/{id}`

**Path Parameters:**
- `id` (string): MongoDB ObjectId as hex string

**Request Body:** (partial update supported)
```json
{
  "price": 899.99,
  "stock": 45
}
```

**Response:** `200 OK`
```json
{
  "_id": "507f1f77bcf86cd799439011",
  "name": "Laptop",
  "price": 899.99,
  "stock": 45
}
```

**Error Response:** `404 Not Found`
```json
{
  "error": "Not found",
  "id": "507f1f77bcf86cd799439011"
}
```

**Example:**
```bash
curl -X PUT http://127.0.0.1:8000/products/507f1f77bcf86cd799439011 \
  -H 'Content-Type: application/json' \
  -d '{"price": 899.99}'
```

### 5. Delete Document

**Endpoint:** `DELETE {prefix}/{id}`

**Path Parameters:**
- `id` (string): MongoDB ObjectId as hex string

**Response:** `204 No Content`

**Error Response:** `404 Not Found`
```json
{
  "error": "Not found",
  "id": "507f1f77bcf86cd799439011"
}
```

**Example:**
```bash
curl -X DELETE http://127.0.0.1:8000/products/507f1f77bcf86cd799439011
```

## Advanced Usage

### Selective Operations

Generate only specific endpoints using the `operations` string or boolean flags:

```python
# Read-only API (only GET endpoints)
app.crud_routes(Product, operations="RL")  # Read + List

# No delete operation (security)
app.crud_routes(Product, operations="CRUL")  # All except Delete

# Create and read only
app.crud_routes(Product, operations="CR")

# Using boolean flags for explicit control
app.crud_routes(Product, create=True, read=True, update=False, delete=False, list=True)
```

### Custom Prefix

Override the default collection name prefix:

```python
app.crud_routes(Product, prefix="/api/v1/inventory/products")
```

This generates endpoints like:
- `GET /api/v1/inventory/products`
- `POST /api/v1/inventory/products`
- etc.

### Custom Tags

Specify OpenAPI tags for documentation:

```python
app.crud_routes(Product, tags=["inventory", "e-commerce", "products"])
```

### Multiple Models

Generate CRUD endpoints for multiple models:

```python
class Product(Document):
    name: str
    price: float

    class Settings:
        name = "products"

class Category(Document):
    name: str
    description: str

    class Settings:
        name = "categories"

app = App(title="E-Commerce API", version="1.0.0")

# Full CRUD for products
app.crud_routes(Product)

# Read-only for categories
app.crud_routes(Category, prefix="/api/categories", operations="RL")
```

This generates:
- Product endpoints: `/products` (all 5 operations)
- Category endpoints: `/api/categories` (read and list only)

### Adding Custom Endpoints

You can still add custom endpoints alongside auto-generated CRUD:

```python
# Generate CRUD endpoints
app.crud_routes(Product)

# Custom search endpoint
@app.get("/products/search")
async def search_products(request):
    query = request.get("query_params", {}).get("q", "")
    products = await Product.find(Product.name.regex(query)).to_list()
    return {
        "status": 200,
        "body": {
            "results": [p.to_dict() for p in products]
        }
    }

# Custom statistics endpoint
@app.get("/stats/products")
async def product_stats(request):
    total = await Product.find().count()
    avg_price = await Product.aggregate([
        {"$group": {"_id": None, "avg": {"$avg": "$price"}}}
    ]).to_list()

    return {
        "status": 200,
        "body": {
            "total_products": total,
            "average_price": avg_price[0]["avg"] if avg_price else 0
        }
    }
```

## Error Handling

All CRUD endpoints include automatic error handling:

- **400 Bad Request**: Invalid request body or validation errors
- **404 Not Found**: Document not found by ID
- **500 Internal Server Error**: Database errors (automatically handled by PyLoop)

## Performance

PyLoop CRUD endpoints benefit from the same performance optimizations as the rest of PyLoop:

- **GIL Released**: Database operations run with GIL released for maximum concurrency
- **Zero-Copy**: Minimal Python object allocation
- **Fast JSON**: Uses sonic-rs for 3-7x faster JSON serialization
- **Connection Pooling**: Reuses MongoDB connections efficiently

## Example Application

See `/Users/chris.cheng/chris-project/data-bridge/examples/pyloop_crud_example.py` for a complete working example.

Run the example:

```bash
python examples/pyloop_crud_example.py
```

Then test the endpoints:

```bash
# Create a product
curl -X POST http://127.0.0.1:8000/products \
  -H 'Content-Type: application/json' \
  -d '{"name": "Laptop", "price": 999.99, "stock": 50}'

# List products
curl http://127.0.0.1:8000/products

# Get product by ID (use ID from create response)
curl http://127.0.0.1:8000/products/{id}

# Update product
curl -X PUT http://127.0.0.1:8000/products/{id} \
  -H 'Content-Type: application/json' \
  -d '{"price": 899.99}'

# Delete product
curl -X DELETE http://127.0.0.1:8000/products/{id}
```

## Testing

Unit tests are available in `tests/test_pyloop_crud.py`:

```bash
pytest tests/test_pyloop_crud.py -v
```

## Limitations

1. **Pagination Cap**: List endpoint is capped at 100 documents per request for performance
2. **Field Filtering**: No built-in field filtering (use custom endpoints for complex queries)
3. **Validation**: Validation happens during `document.save()`, following data-bridge's lazy validation pattern
4. **Internal Fields**: Update endpoint skips fields starting with `_` (internal fields)

## Next Steps

- Phase 4: Request validation and schema inference
- Phase 5: OpenAPI documentation generation
- Phase 6: Rate limiting and middleware support

## See Also

- [PyLoop Documentation](./PYLOOP.md)
- [Document API](./DOCUMENT_API.md)
- [Example: pyloop_crud_example.py](../examples/pyloop_crud_example.py)
