"""
Example usage of Document and EmbeddedDocument with MongoDB-style models.
"""

from datetime import datetime
from typing import Optional

from data_bridge import (
    Document,
    EmbeddedDocument,
    StringField,
    IntField,
    FloatField,
    BoolField,
    ListField,
    DateTimeField,
    ReferenceField,
    EmbeddedDocumentField,
    EmbeddedDocumentListField,
    ObjectIdField,
    GeoPointField,
)


# Define an embedded document for address
class Address(EmbeddedDocument):
    street = StringField(required=True)
    city = StringField(required=True)
    state = StringField(required=True)
    zip_code = StringField(required=True)
    country = StringField(default="USA")
    location = GeoPointField(required=False)  # For geospatial queries


# Define an embedded document for social media profiles
class SocialProfile(EmbeddedDocument):
    platform = StringField(required=True)  # twitter, facebook, linkedin, etc.
    username = StringField(required=True)
    verified = BoolField(default=False)
    followers = IntField(default=0)


# Define the main User document
class User(Document):
    # Basic fields
    username = StringField(required=True, unique=True, max_length=50)
    email = StringField(required=True, unique=True)
    full_name = StringField(required=True)
    age = IntField(required=False)
    is_active = BoolField(default=True)
    
    # Embedded documents
    address = EmbeddedDocumentField(Address, required=False)
    social_profiles = EmbeddedDocumentListField(SocialProfile)
    
    # Timestamps
    created_at = DateTimeField(auto_now_add=True)
    updated_at = DateTimeField(auto_now=True)
    last_login = DateTimeField(required=False)
    
    # Metadata
    tags = ListField(StringField(), default=[])
    settings = DictField(default={})
    
    class Meta:
        collection = "users"
        database = "myapp"
        indexes = [
            {"fields": ["username"], "unique": True},
            {"fields": ["email"], "unique": True},
            {"fields": ["created_at"], "order": -1},
            {"fields": ["address.location"], "type": "2dsphere"},
        ]


# Define a Post document that references User
class Post(Document):
    title = StringField(required=True, max_length=200)
    content = StringField(required=True)
    author = ReferenceField(User, required=True)
    
    # Engagement metrics
    views = IntField(default=0)
    likes = IntField(default=0)
    
    # Nested references
    liked_by = ListField(ReferenceField(User), default=[])
    
    # Timestamps
    created_at = DateTimeField(auto_now_add=True)
    updated_at = DateTimeField(auto_now=True)
    published_at = DateTimeField(required=False)
    
    # Categories and tags
    category = StringField(required=True)
    tags = ListField(StringField(), default=[])
    
    # Status
    is_published = BoolField(default=False)
    is_featured = BoolField(default=False)
    
    class Meta:
        collection = "posts"
        database = "myapp"
        indexes = [
            {"fields": ["author", "-created_at"]},
            {"fields": ["category", "is_published"]},
            {"fields": ["tags"], "type": "text"},
        ]


def demo_usage():
    """Demonstrate usage of Document and EmbeddedDocument."""
    
    # Create embedded documents
    address = Address(
        street="123 Main St",
        city="San Francisco",
        state="CA",
        zip_code="94105",
        location=(37.7749, -122.4194)  # (latitude, longitude)
    )
    
    social_profiles = [
        SocialProfile(
            platform="twitter",
            username="johndoe",
            verified=True,
            followers=1500
        ),
        SocialProfile(
            platform="linkedin",
            username="john-doe",
            verified=False,
            followers=500
        )
    ]
    
    # Create a user document
    user = User(
        username="johndoe",
        email="john@example.com",
        full_name="John Doe",
        age=30,
        address=address,
        social_profiles=social_profiles,
        tags=["developer", "python", "mongodb"],
        settings={"theme": "dark", "notifications": True}
    )
    
    # Query examples (would work with actual MongoDB backend)
    
    # Find users by age range
    young_adults = User.objects().find(
        (User.age >= 18) & (User.age <= 35)
    )
    
    # Find users by location (near San Francisco)
    nearby_users = User.objects().find(
        User.address.location.near((37.7749, -122.4194), max_distance=10000)
    )
    
    # Find users with verified social profiles
    verified_users = User.objects().find(
        User.social_profiles.contains({"verified": True})
    )
    
    # Create a post referencing the user
    post = Post(
        title="Introduction to MongoDB with Python",
        content="This is a comprehensive guide...",
        author=user,  # Reference to user document
        category="technology",
        tags=["mongodb", "python", "database"],
        is_published=True,
        published_at=datetime.utcnow()
    )
    
    # Query posts by author
    user_posts = Post.objects().find(Post.author == user)
    
    # Find published posts in category
    tech_posts = Post.objects().find(
        (Post.category == "technology") & 
        (Post.is_published == True)
    ).sort("-created_at").limit(10)
    
    # Find posts with specific tags
    python_posts = Post.objects().find(
        Post.tags.contains("python")
    )
    
    # Aggregate-like queries
    featured_recent = Post.objects().find(
        (Post.is_featured == True) &
        (Post.created_at >= datetime(2024, 1, 1))
    ).sort("-likes").limit(5)
    
    print(f"Created user: {user}")
    print(f"User address: {user.address.city}, {user.address.state}")
    print(f"Social profiles: {[sp.platform for sp in user.social_profiles]}")


if __name__ == "__main__":
    demo_usage()