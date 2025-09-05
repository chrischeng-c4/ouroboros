"""Tests for Redis key patterns module."""

import pytest

from data_bridge.redis.key_patterns import RedisKeyPattern


class TestRedisKeyPattern:
    """Test RedisKeyPattern class."""


class TestBuildKey:
    """Test build_key method."""
    
    def test_build_key_string_primary_key(self) -> None:
        """Test building key with string primary key."""
        result = RedisKeyPattern.build_key("user:", "123")
        assert result == "user:123"
    
    def test_build_key_integer_primary_key(self) -> None:
        """Test building key with integer primary key."""
        result = RedisKeyPattern.build_key("user:", 123)
        assert result == "user:123"
    
    def test_build_key_uuid_primary_key(self) -> None:
        """Test building key with UUID primary key."""
        import uuid
        user_id = uuid.uuid4()
        result = RedisKeyPattern.build_key("user:", user_id)
        assert result == f"user:{user_id}"
    
    def test_build_key_complex_prefix(self) -> None:
        """Test building key with complex prefix."""
        result = RedisKeyPattern.build_key("app:user:profile:", "john_doe")
        assert result == "app:user:profile:john_doe"
    
    def test_build_key_empty_string_primary_key(self) -> None:
        """Test building key with empty string primary key."""
        result = RedisKeyPattern.build_key("user:", "")
        assert result == "user:"
    
    def test_build_key_zero_primary_key(self) -> None:
        """Test building key with zero primary key."""
        result = RedisKeyPattern.build_key("user:", 0)
        assert result == "user:0"
    
    def test_build_key_boolean_primary_key(self) -> None:
        """Test building key with boolean primary key."""
        result_true = RedisKeyPattern.build_key("flag:", True)
        assert result_true == "flag:True"
        
        result_false = RedisKeyPattern.build_key("flag:", False)
        assert result_false == "flag:False"
    
    def test_build_key_none_primary_key(self) -> None:
        """Test building key with None primary key."""
        result = RedisKeyPattern.build_key("user:", None)
        assert result == "user:None"
    
    def test_build_key_float_primary_key(self) -> None:
        """Test building key with float primary key."""
        result = RedisKeyPattern.build_key("price:", 19.99)
        assert result == "price:19.99"
    
    def test_build_key_special_characters_primary_key(self) -> None:
        """Test building key with special characters in primary key."""
        result = RedisKeyPattern.build_key("user:", "user@example.com")
        assert result == "user:user@example.com"


class TestParseKey:
    """Test parse_key method."""
    
    def test_parse_key_valid_key(self) -> None:
        """Test parsing valid key."""
        result = RedisKeyPattern.parse_key("user:123", "user:")
        assert result == "123"
    
    def test_parse_key_complex_primary_key(self) -> None:
        """Test parsing key with complex primary key."""
        result = RedisKeyPattern.parse_key("user:john_doe_123", "user:")
        assert result == "john_doe_123"
    
    def test_parse_key_complex_prefix(self) -> None:
        """Test parsing key with complex prefix."""
        result = RedisKeyPattern.parse_key("app:user:profile:john_doe", "app:user:profile:")
        assert result == "john_doe"
    
    def test_parse_key_empty_primary_key(self) -> None:
        """Test parsing key with empty primary key."""
        result = RedisKeyPattern.parse_key("user:", "user:")
        assert result == ""
    
    def test_parse_key_uuid_primary_key(self) -> None:
        """Test parsing key with UUID primary key."""
        import uuid
        user_id = str(uuid.uuid4())
        key = f"user:{user_id}"
        result = RedisKeyPattern.parse_key(key, "user:")
        assert result == user_id
    
    def test_parse_key_numeric_primary_key(self) -> None:
        """Test parsing key with numeric primary key."""
        result = RedisKeyPattern.parse_key("user:123", "user:")
        assert result == "123"  # Returns as string
    
    def test_parse_key_special_characters_primary_key(self) -> None:
        """Test parsing key with special characters in primary key."""
        result = RedisKeyPattern.parse_key("user:user@example.com", "user:")
        assert result == "user@example.com"
    
    def test_parse_key_prefix_mismatch_error(self) -> None:
        """Test error when key doesn't match prefix."""
        with pytest.raises(ValueError, match="Key 'post:123' does not match prefix 'user:'"):
            RedisKeyPattern.parse_key("post:123", "user:")
    
    def test_parse_key_partial_prefix_match_error(self) -> None:
        """Test error when key partially matches prefix."""
        with pytest.raises(ValueError, match="Key 'use:123' does not match prefix 'user:'"):
            RedisKeyPattern.parse_key("use:123", "user:")
    
    def test_parse_key_empty_key_error(self) -> None:
        """Test error when key is empty."""
        with pytest.raises(ValueError, match="Key '' does not match prefix 'user:'"):
            RedisKeyPattern.parse_key("", "user:")
    
    def test_parse_key_empty_prefix(self) -> None:
        """Test parsing key with empty prefix."""
        result = RedisKeyPattern.parse_key("user:123", "")
        assert result == "user:123"
    
    def test_parse_key_longer_key_than_prefix(self) -> None:
        """Test parsing key longer than prefix."""
        result = RedisKeyPattern.parse_key("user:profile:123:details", "user:")
        assert result == "profile:123:details"
    
    def test_parse_key_case_sensitivity(self) -> None:
        """Test that parsing is case sensitive."""
        with pytest.raises(ValueError, match="Key 'User:123' does not match prefix 'user:'"):
            RedisKeyPattern.parse_key("User:123", "user:")


class TestBuildPattern:
    """Test build_pattern method."""
    
    def test_build_pattern_simple_prefix(self) -> None:
        """Test building pattern with simple prefix."""
        result = RedisKeyPattern.build_pattern("user:")
        assert result == "user:*"
    
    def test_build_pattern_complex_prefix(self) -> None:
        """Test building pattern with complex prefix."""
        result = RedisKeyPattern.build_pattern("app:user:profile:")
        assert result == "app:user:profile:*"
    
    def test_build_pattern_empty_prefix(self) -> None:
        """Test building pattern with empty prefix."""
        result = RedisKeyPattern.build_pattern("")
        assert result == "*"
    
    def test_build_pattern_prefix_already_with_wildcard(self) -> None:
        """Test building pattern when prefix already contains wildcard."""
        result = RedisKeyPattern.build_pattern("user*:")
        assert result == "user*:*"
    
    def test_build_pattern_prefix_with_special_characters(self) -> None:
        """Test building pattern with special characters in prefix."""
        result = RedisKeyPattern.build_pattern("user@app:")
        assert result == "user@app:*"
    
    def test_build_pattern_numeric_prefix(self) -> None:
        """Test building pattern with numeric characters in prefix."""
        result = RedisKeyPattern.build_pattern("app123:")
        assert result == "app123:*"


class TestValidatePrefix:
    """Test validate_prefix method."""
    
    def test_validate_prefix_valid_simple(self) -> None:
        """Test validation of valid simple prefix."""
        # Should not raise any exception
        RedisKeyPattern.validate_prefix("user:")
    
    def test_validate_prefix_valid_complex(self) -> None:
        """Test validation of valid complex prefix."""
        # Should not raise any exception
        RedisKeyPattern.validate_prefix("app:user:profile:")
    
    def test_validate_prefix_valid_with_numbers(self) -> None:
        """Test validation of valid prefix with numbers."""
        # Should not raise any exception
        RedisKeyPattern.validate_prefix("app123:")
    
    def test_validate_prefix_valid_with_special_chars(self) -> None:
        """Test validation of valid prefix with special characters."""
        # Should not raise any exception
        RedisKeyPattern.validate_prefix("user@app:")
    
    def test_validate_prefix_empty_error(self) -> None:
        """Test error for empty prefix."""
        with pytest.raises(ValueError, match="Prefix cannot be empty"):
            RedisKeyPattern.validate_prefix("")
    
    def test_validate_prefix_no_colon_error(self) -> None:
        """Test error for prefix without colon."""
        with pytest.raises(ValueError, match="Prefix should end with ':'"):
            RedisKeyPattern.validate_prefix("user")
    
    def test_validate_prefix_no_ending_colon_error(self) -> None:
        """Test error for prefix without ending colon."""
        with pytest.raises(ValueError, match="Prefix should end with ':'"):
            RedisKeyPattern.validate_prefix("user:profile")
    
    def test_validate_prefix_with_spaces_error(self) -> None:
        """Test error for prefix containing spaces."""
        with pytest.raises(ValueError, match="Prefix should not contain spaces"):
            RedisKeyPattern.validate_prefix("user profile:")
    
    def test_validate_prefix_spaces_in_middle_error(self) -> None:
        """Test error for prefix with spaces in middle."""
        with pytest.raises(ValueError, match="Prefix should not contain spaces"):
            RedisKeyPattern.validate_prefix("user: profile:")
    
    def test_validate_prefix_only_colon(self) -> None:
        """Test validation of prefix that is only a colon."""
        # Should not raise any exception
        RedisKeyPattern.validate_prefix(":")
    
    def test_validate_prefix_multiple_colons(self) -> None:
        """Test validation of prefix with multiple colons."""
        # Should not raise any exception
        RedisKeyPattern.validate_prefix("app:user:profile:session:")


class TestIntegrationScenarios:
    """Test integration scenarios combining multiple methods."""
    
    def test_build_parse_roundtrip(self) -> None:
        """Test building a key and parsing it back."""
        prefix = "user:"
        primary_key = "123"
        
        # Build key
        built_key = RedisKeyPattern.build_key(prefix, primary_key)
        
        # Parse key back
        parsed_key = RedisKeyPattern.parse_key(built_key, prefix)
        
        assert parsed_key == primary_key
    
    def test_build_parse_roundtrip_complex(self) -> None:
        """Test building and parsing complex key."""
        prefix = "app:user:profile:"
        primary_key = "john_doe_123"
        
        built_key = RedisKeyPattern.build_key(prefix, primary_key)
        parsed_key = RedisKeyPattern.parse_key(built_key, prefix)
        
        assert parsed_key == primary_key
    
    def test_build_parse_roundtrip_special_characters(self) -> None:
        """Test building and parsing key with special characters."""
        prefix = "user:"
        primary_key = "user@example.com"
        
        built_key = RedisKeyPattern.build_key(prefix, primary_key)
        parsed_key = RedisKeyPattern.parse_key(built_key, prefix)
        
        assert parsed_key == primary_key
    
    def test_validate_build_pattern_workflow(self) -> None:
        """Test typical workflow: validate prefix, build pattern."""
        prefix = "user:"
        
        # Validate prefix (should not raise)
        RedisKeyPattern.validate_prefix(prefix)
        
        # Build pattern
        pattern = RedisKeyPattern.build_pattern(prefix)
        assert pattern == "user:*"
    
    def test_full_workflow(self) -> None:
        """Test complete workflow with all methods."""
        prefix = "session:"
        primary_key = "abc123"
        
        # 1. Validate prefix
        RedisKeyPattern.validate_prefix(prefix)
        
        # 2. Build key
        key = RedisKeyPattern.build_key(prefix, primary_key)
        assert key == "session:abc123"
        
        # 3. Build pattern for scanning
        pattern = RedisKeyPattern.build_pattern(prefix)
        assert pattern == "session:*"
        
        # 4. Parse key back
        parsed_key = RedisKeyPattern.parse_key(key, prefix)
        assert parsed_key == primary_key


class TestTypeHints:
    """Test that methods work with various types as documented."""
    
    def test_build_key_type_variations(self) -> None:
        """Test build_key with various types for primary_key."""
        test_cases = [
            ("string", "user:string"),
            (123, "user:123"),
            (123.45, "user:123.45"),
            (True, "user:True"),
            (False, "user:False"),
            (None, "user:None"),
        ]
        
        for primary_key, expected in test_cases:
            result = RedisKeyPattern.build_key("user:", primary_key)
            assert result == expected
    
    def test_parse_key_always_returns_string(self) -> None:
        """Test that parse_key always returns string regardless of input type."""
        # Even if we built with int, parsing returns string
        key = RedisKeyPattern.build_key("user:", 123)
        parsed = RedisKeyPattern.parse_key(key, "user:")
        assert isinstance(parsed, str)
        assert parsed == "123"


class TestEdgeCases:
    """Test edge cases and error conditions."""
    
    def test_prefix_longer_than_key(self) -> None:
        """Test when prefix is longer than key."""
        with pytest.raises(ValueError, match="does not match prefix"):
            RedisKeyPattern.parse_key("u", "user:")
    
    def test_prefix_same_length_as_key_but_different(self) -> None:
        """Test when prefix same length as key but different."""
        with pytest.raises(ValueError, match="does not match prefix"):
            RedisKeyPattern.parse_key("post", "user")
    
    def test_unicode_characters(self) -> None:
        """Test with unicode characters."""
        prefix = "user:"
        primary_key = "用户123"
        
        key = RedisKeyPattern.build_key(prefix, primary_key)
        assert key == "user:用户123"
        
        parsed = RedisKeyPattern.parse_key(key, prefix)
        assert parsed == primary_key
    
    def test_very_long_keys(self) -> None:
        """Test with very long keys."""
        prefix = "user:"
        primary_key = "x" * 1000
        
        key = RedisKeyPattern.build_key(prefix, primary_key)
        parsed = RedisKeyPattern.parse_key(key, prefix)
        assert parsed == primary_key
    
    def test_newlines_in_primary_key(self) -> None:
        """Test with newlines in primary key."""
        prefix = "user:"
        primary_key = "line1\nline2"
        
        key = RedisKeyPattern.build_key(prefix, primary_key)
        parsed = RedisKeyPattern.parse_key(key, prefix)
        assert parsed == primary_key