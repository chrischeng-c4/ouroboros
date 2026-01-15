//! Query builder for MongoDB operations

use bson::Document as BsonDocument;
use ouroboros_common::Result;
use mongodb::{Collection, Database};

/// Query builder for MongoDB find operations
pub struct QueryBuilder {
    collection_name: String,
    filter: BsonDocument,
    sort: Option<BsonDocument>,
    skip: Option<u64>,
    limit: Option<i64>,
}

impl QueryBuilder {
    /// Create a new query builder
    pub fn new(collection_name: impl Into<String>) -> Self {
        Self {
            collection_name: collection_name.into(),
            filter: BsonDocument::new(),
            sort: None,
            skip: None,
            limit: None,
        }
    }

    /// Set the filter document
    pub fn filter(mut self, filter: BsonDocument) -> Self {
        self.filter = filter;
        self
    }

    /// Set the sort order
    pub fn sort(mut self, sort: BsonDocument) -> Self {
        self.sort = Some(sort);
        self
    }

    /// Set the number of documents to skip
    pub fn skip(mut self, skip: u64) -> Self {
        self.skip = Some(skip);
        self
    }

    /// Set the maximum number of documents to return
    pub fn limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Get the collection name
    pub fn collection_name(&self) -> &str {
        &self.collection_name
    }

    /// Get the filter document
    pub fn get_filter(&self) -> &BsonDocument {
        &self.filter
    }

    /// Get the sort document
    pub fn get_sort(&self) -> Option<&BsonDocument> {
        self.sort.as_ref()
    }

    /// Get the skip value
    pub fn get_skip(&self) -> Option<u64> {
        self.skip
    }

    /// Get the limit value
    pub fn get_limit(&self) -> Option<i64> {
        self.limit
    }

    /// Execute the query and return all matching documents
    pub async fn to_list(self, db: &Database) -> Result<Vec<BsonDocument>> {
        let collection: Collection<BsonDocument> = db.collection(&self.collection_name);

        let mut cursor = collection.find(self.filter).await?;

        let mut results = Vec::new();
        while cursor.advance().await? {
            results.push(cursor.deserialize_current()?);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bson::doc;

    #[test]
    fn test_query_builder_new() {
        let qb = QueryBuilder::new("users");
        assert_eq!(qb.collection_name(), "users");
        assert!(qb.get_filter().is_empty());
        assert!(qb.get_sort().is_none());
        assert!(qb.get_skip().is_none());
        assert!(qb.get_limit().is_none());
    }

    #[test]
    fn test_query_builder_filter() {
        let filter = doc! { "email": "test@example.com" };
        let qb = QueryBuilder::new("users").filter(filter.clone());
        assert_eq!(qb.get_filter(), &filter);
    }

    #[test]
    fn test_query_builder_sort() {
        let sort = doc! { "created_at": -1 };
        let qb = QueryBuilder::new("users").sort(sort.clone());
        assert_eq!(qb.get_sort(), Some(&sort));
    }

    #[test]
    fn test_query_builder_skip() {
        let qb = QueryBuilder::new("users").skip(10);
        assert_eq!(qb.get_skip(), Some(10));
    }

    #[test]
    fn test_query_builder_limit() {
        let qb = QueryBuilder::new("users").limit(20);
        assert_eq!(qb.get_limit(), Some(20));
    }

    #[test]
    fn test_query_builder_chaining() {
        let filter = doc! { "active": true };
        let sort = doc! { "name": 1 };

        let qb = QueryBuilder::new("users")
            .filter(filter.clone())
            .sort(sort.clone())
            .skip(5)
            .limit(10);

        assert_eq!(qb.collection_name(), "users");
        assert_eq!(qb.get_filter(), &filter);
        assert_eq!(qb.get_sort(), Some(&sort));
        assert_eq!(qb.get_skip(), Some(5));
        assert_eq!(qb.get_limit(), Some(10));
    }

    #[test]
    fn test_query_builder_complex_filter() {
        let filter = doc! {
            "$and": [
                { "age": { "$gte": 18 } },
                { "status": "active" },
                { "email": { "$regex": "@example.com$" } }
            ]
        };
        let qb = QueryBuilder::new("users").filter(filter.clone());
        assert_eq!(qb.get_filter(), &filter);
    }
}
