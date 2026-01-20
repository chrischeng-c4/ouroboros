//! Integration tests for Django type provider

use argus::types::{
    DjangoTypeProvider, FrameworkTypeProvider, Type,
};

/// Helper to create a test Django provider with models
fn setup_django_provider_with_models(source: &str) -> DjangoTypeProvider {
    let mut provider = DjangoTypeProvider::new();
    provider.parse_models_from_source(source, "myapp.models");
    provider
}

#[test]
fn test_parse_simple_model() {
    let source = r#"
from django.db import models

class User(models.Model):
    name = models.CharField(max_length=100)
    email = models.EmailField()
    age = models.IntegerField()
    is_active = models.BooleanField(default=True)
"#;

    let provider = setup_django_provider_with_models(source);

    // Check model is registered
    assert!(provider.get_model("User").is_some(), "User model should be registered");

    let user_model = provider.get_model("User").unwrap();
    assert_eq!(user_model.fields.len(), 4, "Should have 4 fields");

    // Check field types
    assert!(user_model.fields.contains_key("name"), "Should have name field");
    assert!(user_model.fields.contains_key("email"), "Should have email field");
    assert!(user_model.fields.contains_key("age"), "Should have age field");
    assert!(user_model.fields.contains_key("is_active"), "Should have is_active field");
}

#[test]
fn test_get_model_field_types() {
    let source = r#"
class Product(models.Model):
    title = models.CharField(max_length=200)
    price = models.FloatField()
    description = models.TextField()
    created_at = models.DateTimeField(auto_now_add=True)
"#;

    let provider = setup_django_provider_with_models(source);

    // Test get_attribute_type for model fields
    let product_type = Type::Instance {
        name: "Product".to_string(),
        module: None,
        type_args: Vec::new(),
    };

    // Check field types via get_attribute_type
    let title_type = provider.get_attribute_type(&product_type, "title");
    assert!(matches!(title_type, Some(Type::Str)), "title should be str");

    let price_type = provider.get_attribute_type(&product_type, "price");
    assert!(matches!(price_type, Some(Type::Float)), "price should be float");

    let description_type = provider.get_attribute_type(&product_type, "description");
    assert!(matches!(description_type, Some(Type::Str)), "description should be str");

    let created_at_type = provider.get_attribute_type(&product_type, "created_at");
    assert!(matches!(created_at_type, Some(Type::Instance { .. })), "created_at should be datetime instance");
}

#[test]
fn test_queryset_filter_method() {
    let source = r#"
class Article(models.Model):
    title = models.CharField(max_length=100)
    published = models.BooleanField()
"#;

    let provider = setup_django_provider_with_models(source);

    // Test QuerySet.filter() method
    let queryset_type = Type::Instance {
        name: "ArticleQuerySet".to_string(),
        module: None,
        type_args: Vec::new(),
    };

    let filter_sig = provider.get_method_signature(&queryset_type, "filter");
    assert!(filter_sig.is_some(), "filter() should have a signature");

    let sig = filter_sig.unwrap();
    // Should return ArticleQuerySet
    if let Type::Instance { name, .. } = &sig.return_type {
        assert_eq!(name, "ArticleQuerySet", "filter() should return ArticleQuerySet");
    } else {
        panic!("filter() should return Instance type");
    }
}

#[test]
fn test_queryset_get_method() {
    let source = r#"
class Comment(models.Model):
    text = models.TextField()
    author = models.CharField(max_length=50)
"#;

    let provider = setup_django_provider_with_models(source);

    // Test QuerySet.get() method
    let queryset_type = Type::Instance {
        name: "CommentQuerySet".to_string(),
        module: None,
        type_args: Vec::new(),
    };

    let get_sig = provider.get_method_signature(&queryset_type, "get");
    assert!(get_sig.is_some(), "get() should have a signature");

    let sig = get_sig.unwrap();
    // Should return Comment instance
    if let Type::Instance { name, .. } = &sig.return_type {
        assert_eq!(name, "Comment", "get() should return Comment instance");
    } else {
        panic!("get() should return Instance type");
    }
}

#[test]
fn test_queryset_count_and_exists() {
    let source = r#"
class Item(models.Model):
    name = models.CharField(max_length=50)
"#;

    let provider = setup_django_provider_with_models(source);

    let queryset_type = Type::Instance {
        name: "ItemQuerySet".to_string(),
        module: None,
        type_args: Vec::new(),
    };

    // Test count() returns int
    let count_sig = provider.get_method_signature(&queryset_type, "count");
    assert!(count_sig.is_some(), "count() should have a signature");
    assert!(matches!(count_sig.unwrap().return_type, Type::Int), "count() should return int");

    // Test exists() returns bool
    let exists_sig = provider.get_method_signature(&queryset_type, "exists");
    assert!(exists_sig.is_some(), "exists() should have a signature");
    assert!(matches!(exists_sig.unwrap().return_type, Type::Bool), "exists() should return bool");
}

#[test]
fn test_queryset_first_returns_optional() {
    let source = r#"
class Task(models.Model):
    title = models.CharField(max_length=100)
    completed = models.BooleanField()
"#;

    let provider = setup_django_provider_with_models(source);

    let queryset_type = Type::Instance {
        name: "TaskQuerySet".to_string(),
        module: None,
        type_args: Vec::new(),
    };

    // Test first() returns Optional[Task]
    let first_sig = provider.get_method_signature(&queryset_type, "first");
    assert!(first_sig.is_some(), "first() should have a signature");

    let sig = first_sig.unwrap();
    if let Type::Optional(inner) = &sig.return_type {
        if let Type::Instance { name, .. } = inner.as_ref() {
            assert_eq!(name, "Task", "first() should return Optional[Task]");
        } else {
            panic!("first() inner type should be Instance");
        }
    } else {
        panic!("first() should return Optional type");
    }
}

#[test]
fn test_model_with_foreign_key() {
    let source = r#"
class Author(models.Model):
    name = models.CharField(max_length=100)

class Book(models.Model):
    title = models.CharField(max_length=200)
    author = models.ForeignKey(Author, on_delete=models.CASCADE)
    published_date = models.DateField()
"#;

    let provider = setup_django_provider_with_models(source);

    // Check both models are registered
    assert!(provider.get_model("Author").is_some());
    assert!(provider.get_model("Book").is_some());

    let book_model = provider.get_model("Book").unwrap();
    assert_eq!(book_model.relations.len(), 1, "Book should have 1 relation");

    let author_relation = &book_model.relations[0];
    assert_eq!(author_relation.name, "author");
    assert_eq!(author_relation.related_model, "Author");

    // Test get_attribute_type for ForeignKey
    let book_type = Type::Instance {
        name: "Book".to_string(),
        module: None,
        type_args: Vec::new(),
    };

    let author_type = provider.get_attribute_type(&book_type, "author");
    assert!(author_type.is_some(), "author field should exist");

    if let Some(Type::Instance { name, .. }) = author_type {
        assert_eq!(name, "Author", "author field should be Author instance");
    } else {
        panic!("author field should be Instance type");
    }
}

#[test]
fn test_model_save_method() {
    let source = r#"
class Profile(models.Model):
    bio = models.TextField()
    avatar = models.CharField(max_length=200)
"#;

    let provider = setup_django_provider_with_models(source);

    // Test Model.save() method (not QuerySet)
    let profile_type = Type::Instance {
        name: "Profile".to_string(),
        module: None,
        type_args: Vec::new(),
    };

    let save_sig = provider.get_method_signature(&profile_type, "save");
    assert!(save_sig.is_some(), "save() should have a signature for model instances");

    let sig = save_sig.unwrap();
    assert!(matches!(sig.return_type, Type::None), "save() should return None");
}

#[test]
fn test_queryset_chaining() {
    let source = r#"
class Order(models.Model):
    customer = models.CharField(max_length=100)
    total = models.FloatField()
    status = models.CharField(max_length=20)
"#;

    let provider = setup_django_provider_with_models(source);

    let queryset_type = Type::Instance {
        name: "OrderQuerySet".to_string(),
        module: None,
        type_args: Vec::new(),
    };

    // Test that chaining methods (filter, exclude, order_by) all return QuerySet
    let methods = ["filter", "exclude", "order_by", "select_related", "prefetch_related"];

    for method in &methods {
        let sig = provider.get_method_signature(&queryset_type, method);
        assert!(sig.is_some(), "{} should have a signature", method);

        if let Type::Instance { name, .. } = &sig.unwrap().return_type {
            assert_eq!(name, "OrderQuerySet", "{} should return OrderQuerySet for chaining", method);
        } else {
            panic!("{} should return Instance type", method);
        }
    }
}

#[test]
fn test_get_or_create_returns_tuple() {
    let source = r#"
class Setting(models.Model):
    key = models.CharField(max_length=50)
    value = models.TextField()
"#;

    let provider = setup_django_provider_with_models(source);

    let queryset_type = Type::Instance {
        name: "SettingQuerySet".to_string(),
        module: None,
        type_args: Vec::new(),
    };

    let sig = provider.get_method_signature(&queryset_type, "get_or_create");
    assert!(sig.is_some(), "get_or_create() should have a signature");

    let return_type = &sig.unwrap().return_type;
    if let Type::Tuple(elements) = return_type {
        assert_eq!(elements.len(), 2, "get_or_create() should return tuple of 2 elements");

        // First element should be Setting instance
        if let Type::Instance { name, .. } = &elements[0] {
            assert_eq!(name, "Setting");
        } else {
            panic!("First element should be Instance");
        }

        // Second element should be bool
        assert!(matches!(elements[1], Type::Bool), "Second element should be bool");
    } else {
        panic!("get_or_create() should return Tuple type");
    }
}
