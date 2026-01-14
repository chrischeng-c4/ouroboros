## 1. Implementation
- [x] 1.1 Update `Table.find()` in `python/ouroboros/postgres/table.py` to accept `fetch_links: bool = False`.
- [x] 1.2 Implement logic in `find()` to inspect `cls._relationships` and apply `selectinload` for all relationships when `fetch_links=True`.
- [x] 1.3 Update `Table.find_one()` in `python/ouroboros/postgres/table.py` to accept `fetch_links: bool = False` and pass it to `find()`.

## 2. Testing
- [x] 2.1 Add a test case verifying `fetch_links=True` loads relationships without N+1 queries.
- [x] 2.2 Verify `options(selectinload(...))` still works as intended.
