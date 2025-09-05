"""
Example showcasing the new MongoDB field update operations.
This demonstrates the comprehensive update operations support added to each field type.
"""

from data_bridge.base.fields import (
    BoolField,
    DictField,
    FloatField,
    IntField,
    ListField,
    StringField,
    UpdateExpression,
)
from data_bridge.mongo.translator import MongoUpdateTranslator


def demonstrate_field_operations():
    """Demonstrate update operations for each field type."""
    
    print("=== MongoDB Field Update Operations Demo ===\n")
    
    # Create field instances with names (simulating field descriptors)
    name_field = StringField()
    name_field.name = "name"
    
    age_field = IntField()
    age_field.name = "age"
    
    score_field = FloatField()
    score_field.name = "score"
    
    active_field = BoolField()
    active_field.name = "active"
    
    tags_field = ListField(str)
    tags_field.name = "tags"
    
    metadata_field = DictField()
    metadata_field.name = "metadata"
    
    print("1. Basic Field Operations (all field types):")
    print("   - set(), unset(), rename()")
    
    # Basic operations available on all fields
    basic_updates = [
        name_field.set("John Doe"),
        age_field.unset(),
        score_field.rename("new_score")
    ]
    
    result = MongoUpdateTranslator.translate(basic_updates)
    print(f"   MongoDB Update: {result}")
    print()
    
    print("2. Numeric Field Operations (IntField, FloatField):")
    print("   - inc(), mul(), min(), max()")
    
    # Numeric operations
    numeric_updates = [
        age_field.inc(1),           # Increment age by 1
        score_field.mul(1.5),       # Multiply score by 1.5
        age_field.max(100),         # Set age to max of current and 100
        score_field.min(0.0)        # Set score to min of current and 0.0
    ]
    
    result = MongoUpdateTranslator.translate(numeric_updates)
    print(f"   MongoDB Update: {result}")
    print()
    
    print("3. Boolean Field Operations:")
    print("   - toggle()")
    
    # Boolean operations
    bool_updates = [
        active_field.toggle()       # Flip boolean value
    ]
    
    result = MongoUpdateTranslator.translate(bool_updates)
    print(f"   MongoDB Update: {result}")
    print()
    
    print("4. Array Field Operations (ListField):")
    print("   - push(), push_all(), pull(), pull_all(), add_to_set(), pop()")
    
    # Array operations - basic
    array_updates_basic = [
        tags_field.push("new_tag"),                    # Add single element
        tags_field.pull("old_tag"),                    # Remove matching elements
        tags_field.add_to_set("unique_tag"),           # Add unique element
        tags_field.pop(1)                              # Remove last element
    ]
    
    result = MongoUpdateTranslator.translate(array_updates_basic)
    print(f"   MongoDB Update (Basic): {result}")
    print()
    
    print("5. Advanced Array Operations with Modifiers:")
    print("   - push() with position, slice, sort modifiers")
    print("   - push_all() and add_to_set_each() with $each")
    
    # Array operations - advanced
    array_updates_advanced = [
        # Push with position (insert at beginning)
        tags_field.push("priority_tag", position=0),
        
        # Push with slice (keep only last 10 items)
        tags_field.push("item", slice=10),
        
        # Push with sort (maintain sorted order)
        tags_field.push("sorted_item", sort=1),
        
        # Push multiple items at once
        tags_field.push_all(["tag1", "tag2", "tag3"]),
        
        # Add multiple unique items
        tags_field.add_to_set_each(["unique1", "unique2"])
    ]
    
    # Translate each separately to show the different structures
    for i, update in enumerate(array_updates_advanced):
        result = MongoUpdateTranslator.translate([update])
        print(f"   Update {i+1}: {result}")
    print()
    
    print("6. Dict Field Nested Operations:")
    print("   - set_field(), unset_field(), inc_field()")
    
    # Dict field nested operations
    dict_updates = [
        metadata_field.set_field("config.theme", "dark"),      # Set nested field
        metadata_field.unset_field("old_setting"),             # Remove nested field
        metadata_field.inc_field("stats.count", 1),            # Increment nested number
        metadata_field.set_field("user.preferences.lang", "en") # Deep nested set
    ]
    
    result = MongoUpdateTranslator.translate(dict_updates)
    print(f"   MongoDB Update: {result}")
    print()
    
    print("7. Complex Combined Operations:")
    print("   Multiple field types in single update")
    
    # Complex combined update
    combined_updates = [
        name_field.set("Updated Name"),
        age_field.inc(1),
        score_field.mul(2.0),
        active_field.toggle(),
        tags_field.push("combined_tag", position=0, slice=5),
        metadata_field.set_field("last_updated", "2024-01-01"),
        metadata_field.inc_field("update_count", 1)
    ]
    
    result = MongoUpdateTranslator.translate(combined_updates)
    print(f"   MongoDB Update: {result}")
    print()
    
    print("8. Type-Safe Usage Examples:")
    print("   Demonstrating type safety and IDE support")
    
    # Show type safety
    int_field = IntField()
    int_field.name = "count"
    
    float_field = FloatField() 
    float_field.name = "rate"
    
    # These show proper typing
    int_updates = [
        int_field.inc(5),           # int parameter
        int_field.mul(2),           # int parameter
        float_field.inc(1.5),       # float parameter  
        float_field.mul(0.8)        # float parameter
    ]
    
    result = MongoUpdateTranslator.translate(int_updates)
    print(f"   Typed Updates: {result}")
    
    print("\n=== Demo Complete ===")
    print("All field types now support appropriate MongoDB update operations!")
    print("Each operation is type-safe and generates the correct MongoDB update document.")


if __name__ == "__main__":
    demonstrate_field_operations()