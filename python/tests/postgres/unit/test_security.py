"""
Unit tests for security validation.

Tests security validation of table names, column names, and SQL injection prevention.
These tests verify that the Python layer properly validates inputs before passing
them to the Rust engine.
"""
import pytest
from ouroboros.postgres import Table, Column
from ouroboros.test import expect


class TestTableNameValidation:
    """Test table name security validation."""

    def test_valid_table_name(self):
        """Test valid table name is accepted."""

        class Users(Table):
            name: str

            class Settings:
                table_name = "users"

        expect(Users._table_name).to_equal("users")

    def test_valid_table_name_with_underscore(self):
        """Test table name with underscores is accepted."""

        class UserProfiles(Table):
            name: str

            class Settings:
                table_name = "user_profiles"

        expect(UserProfiles._table_name).to_equal("user_profiles")

    def test_valid_table_name_with_numbers(self):
        """Test table name with numbers is accepted."""

        class Orders(Table):
            name: str

            class Settings:
                table_name = "orders_2024"

        expect(Orders._table_name).to_equal("orders_2024")

    def test_schema_qualified_table_name(self):
        """Test schema-qualified table names work correctly."""

        class Users(Table):
            name: str

            class Settings:
                table_name = "users"
                schema = "auth"

        # Schema is stored separately, not in table_name
        expect(Users._table_name).to_equal("users")
        expect(Users._schema).to_equal("auth")
        expect(Users.__table_name__()).to_equal("auth.users")

    def test_table_name_case_preserved(self):
        """Test table name case is preserved."""

        class Users(Table):
            name: str

            class Settings:
                table_name = "MyUsers"

        # PostgreSQL will lowercase unless quoted, but we preserve the case
        expect(Users._table_name).to_equal("MyUsers")


class TestColumnNameValidation:
    """Test column name security validation."""

    def test_valid_column_names(self):
        """Test valid column names are accepted."""

        class User(Table):
            first_name: str
            last_name: str
            email_address: str
            age: int

        expect("first_name" in User._columns).to_be_true()
        expect("last_name" in User._columns).to_be_true()
        expect("email_address" in User._columns).to_be_true()
        expect("age" in User._columns).to_be_true()

    def test_column_name_with_numbers(self):
        """Test column names with numbers are accepted."""

        class Product(Table):
            name: str
            price_v2: float

        expect("price_v2" in Product._columns).to_be_true()

    def test_column_name_case_preserved(self):
        """Test column name case is preserved."""

        class User(Table):
            firstName: str
            lastName: str

        # Case should be preserved
        expect("firstName" in User._columns).to_be_true()
        expect("lastName" in User._columns).to_be_true()


class TestSQLInjectionPrevention:
    """Test SQL injection prevention in various contexts."""

    def test_semicolon_in_table_name(self):
        """Test semicolon in table name (SQL injection attempt)."""
        # This should be caught at the Rust validation layer when the table is used
        # Python layer allows it, but Rust will reject it

        class Users(Table):
            name: str

            class Settings:
                table_name = "users; DROP TABLE users--"

        # Python allows setting it, but Rust engine would reject
        expect(Users._table_name).to_equal("users; DROP TABLE users--")

    def test_comment_in_table_name(self):
        """Test SQL comment in table name."""

        class Users(Table):
            name: str

            class Settings:
                table_name = "users--comment"

        # Python allows it, validation happens at Rust layer
        expect(Users._table_name).to_equal("users--comment")

    def test_special_chars_in_filter_value(self):
        """Test special characters in filter values are safe."""
        # Values are parameterized, so should be safe

        class User(Table):
            name: str
            email: str

        # This should generate parameterized SQL
        expr = User.email == "test'; DROP TABLE users--"

        expect(expr.value).to_equal("test'; DROP TABLE users--")
        # The to_sql() should use parameterized queries
        sql, params = expr.to_sql()
        expect("$1" in sql).to_be_true()  # Parameterized
        expect(params[0]).to_equal("test'; DROP TABLE users--")

    def test_sql_keywords_in_values(self):
        """Test SQL keywords in values are safe."""

        class User(Table):
            name: str
            bio: str

        expr = User.bio.contains("SELECT * FROM")

        sql, params = expr.to_sql()
        # Should be parameterized
        expect("$1" in sql).to_be_true()
        expect("SELECT * FROM" in params[0]).to_be_true()

    def test_union_injection_in_values(self):
        """Test UNION injection attempt in values."""

        class User(Table):
            name: str

        expr = User.name == "admin' UNION SELECT password FROM users--"

        sql, params = expr.to_sql()
        # Should be safely parameterized
        expect("$1" in sql).to_be_true()
        expect("UNION" in params[0]).to_be_true()  # Treated as literal value


class TestIdentifierValidation:
    """Test identifier validation (table/column names)."""

    def test_empty_table_name(self):
        """Test empty table name uses class name."""

        class User(Table):
            name: str

            class Settings:
                table_name = ""

        # Should default to lowercase class name
        expect(User._table_name).to_equal("user")

    def test_long_identifier(self):
        """Test very long identifier names."""
        # PostgreSQL has a limit of 63 characters for identifiers

        long_name = "a" * 100

        class MyTable(Table):
            name: str

            class Settings:
                table_name = long_name

        # Python allows it, but PostgreSQL might truncate
        expect(MyTable._table_name).to_equal(long_name)

    def test_reserved_words_as_column_names(self):
        """Test SQL reserved words as column names."""
        # These are technically valid if quoted

        class MyTable(Table):
            select: str  # SQL keyword
            from_: str  # SQL keyword (using Python convention)
            where: str  # SQL keyword

        # Python allows it
        expect("select" in MyTable._columns).to_be_true()
        expect("from_" in MyTable._columns).to_be_true()
        expect("where" in MyTable._columns).to_be_true()

    def test_unicode_in_identifiers(self):
        """Test unicode characters in identifiers."""

        class User(Table):
            name: str

            class Settings:
                table_name = "用户"  # Chinese characters

        # Python allows it
        expect(User._table_name).to_equal("用户")


class TestSchemaValidation:
    """Test schema name validation."""

    def test_valid_schema_name(self):
        """Test valid schema names."""

        class User(Table):
            name: str

            class Settings:
                schema = "public"

        expect(User._schema).to_equal("public")

    def test_custom_schema_name(self):
        """Test custom schema names."""

        class User(Table):
            name: str

            class Settings:
                schema = "auth"

        expect(User._schema).to_equal("auth")

    def test_schema_with_underscore(self):
        """Test schema name with underscore."""

        class User(Table):
            name: str

            class Settings:
                schema = "my_schema"

        expect(User._schema).to_equal("my_schema")

    def test_default_schema(self):
        """Test default schema is 'public'."""

        class User(Table):
            name: str

        expect(User._schema).to_equal("public")


class TestPrimaryKeyValidation:
    """Test primary key configuration validation."""

    def test_valid_primary_key(self):
        """Test valid primary key name."""

        class Product(Table):
            sku: str

            class Settings:
                primary_key = "sku"

        expect(Product._primary_key).to_equal("sku")

    def test_default_primary_key(self):
        """Test default primary key is 'id'."""

        class User(Table):
            name: str

        expect(User._primary_key).to_equal("id")

    def test_numeric_primary_key(self):
        """Test numeric primary key name."""

        class User(Table):
            name: str

            class Settings:
                primary_key = "user_id"

        expect(User._primary_key).to_equal("user_id")


class TestQueryParameterization:
    """Test that queries use parameterization for safety."""

    def test_filter_uses_parameters(self):
        """Test filter values are parameterized."""

        class User(Table):
            email: str

        expr = User.email == "test@example.com"
        sql, params = expr.to_sql()

        # Should use $1 placeholder
        expect("$1" in sql).to_be_true()
        expect(params).to_equal(["test@example.com"])

    def test_multiple_filters_parameterized(self):
        """Test multiple filters use sequential parameters."""

        class User(Table):
            name: str
            age: int

        query = User.find(User.name == "Alice", User.age > 25)
        where, params = query._build_where_clause()

        # Should use $1, $2
        expect("$1" in where).to_be_true()
        expect("$2" in where).to_be_true()
        expect(len(params)).to_equal(2)

    def test_in_operator_parameterized(self):
        """Test IN operator uses parameterization."""

        class User(Table):
            city: str

        expr = User.city.in_(["NYC", "LA", "SF"])
        sql, params = expr.to_sql()

        # Should use $1, $2, $3
        expect("IN" in sql).to_be_true()
        expect("$1" in sql).to_be_true()
        expect("$2" in sql).to_be_true()
        expect("$3" in sql).to_be_true()
        expect(params).to_equal(["NYC", "LA", "SF"])

    def test_like_operator_parameterized(self):
        """Test LIKE operator uses parameterization."""

        class User(Table):
            email: str

        expr = User.email.like("%@example.com")
        sql, params = expr.to_sql()

        # Should be parameterized
        expect("$1" in sql).to_be_true()
        expect(params).to_equal(["%@example.com"])

    def test_between_parameterized(self):
        """Test BETWEEN uses parameterization."""

        class User(Table):
            age: int

        expr = User.age.between(18, 65)
        sql, params = expr.to_sql()

        # Should use $1 and $2
        expect("BETWEEN $1 AND $2" in sql).to_be_true()
        expect(params).to_equal([18, 65])


class TestInputSanitization:
    """Test input sanitization and validation."""

    def test_null_byte_in_string(self):
        """Test null bytes in string values."""

        class User(Table):
            name: str

        # Null byte should be treated as regular value
        expr = User.name == "test\x00value"

        sql, params = expr.to_sql()
        expect(params[0]).to_equal("test\x00value")

    def test_newline_in_string(self):
        """Test newlines in string values are safe."""

        class User(Table):
            bio: str

        expr = User.bio == "Line 1\nLine 2"

        sql, params = expr.to_sql()
        expect(params[0]).to_equal("Line 1\nLine 2")

    def test_quote_in_string(self):
        """Test quotes in string values are safe."""

        class User(Table):
            name: str

        expr = User.name == "O'Brien"

        sql, params = expr.to_sql()
        # Parameterization makes this safe
        expect(params[0]).to_equal("O'Brien")

    def test_backslash_in_string(self):
        """Test backslashes in string values."""

        class User(Table):
            path: str

        expr = User.path == "C:\\Users\\Admin"

        sql, params = expr.to_sql()
        expect(params[0]).to_equal("C:\\Users\\Admin")


class TestUnicodeValidation:
    """Test comprehensive Unicode handling in identifiers and values."""

    def test_unicode_table_name_sql_generation(self):
        """Test that Unicode table names generate correct SQL with proper quoting."""

        class ChineseUsers(Table):
            name: str

            class Settings:
                table_name = "用户表"  # Chinese: "User Table"

        class ArabicUsers(Table):
            name: str

            class Settings:
                table_name = "المستخدمون"  # Arabic: "Users"

        class JapaneseUsers(Table):
            name: str

            class Settings:
                table_name = "ユーザー"  # Japanese: "Users"

        # Verify table names are preserved
        expect(ChineseUsers._table_name).to_equal("用户表")
        expect(ArabicUsers._table_name).to_equal("المستخدمون")
        expect(JapaneseUsers._table_name).to_equal("ユーザー")

        # Verify full qualified name generation
        expect(ChineseUsers.__table_name__()).to_equal('public."用户表"')
        expect(ArabicUsers.__table_name__()).to_equal('public."المستخدمون"')
        expect(JapaneseUsers.__table_name__()).to_equal('public."ユーザー"')

    def test_unicode_column_name_sql_generation(self):
        """Test column filtering with Unicode column names."""

        class User(Table):
            姓名: str  # Chinese: "name"
            年龄: int  # Chinese: "age"
            邮箱: str  # Chinese: "email"

        # Verify columns are registered
        expect("姓名" in User._columns).to_be_true()
        expect("年龄" in User._columns).to_be_true()
        expect("邮箱" in User._columns).to_be_true()

        # Test filter expression with Unicode column
        expr = User.姓名 == "Alice"
        sql, params = expr.to_sql()

        # Should use parameterization and quote column name
        expect('"姓名"' in sql).to_be_true()
        expect("$1" in sql).to_be_true()
        expect(params).to_equal(["Alice"])

    def test_unicode_filter_values(self):
        """Test filters with Unicode string values (Chinese, Arabic, Japanese, etc.)."""

        class User(Table):
            name: str
            city: str

        # Chinese
        expr_chinese = User.name == "张三"  # Common Chinese name
        sql, params = expr_chinese.to_sql()
        expect(params[0]).to_equal("张三")

        # Arabic
        expr_arabic = User.name == "محمد"  # Muhammad in Arabic
        sql, params = expr_arabic.to_sql()
        expect(params[0]).to_equal("محمد")

        # Japanese (Hiragana, Katakana, Kanji)
        expr_japanese = User.name == "田中太郎"  # Tanaka Taro
        sql, params = expr_japanese.to_sql()
        expect(params[0]).to_equal("田中太郎")

        # Korean
        expr_korean = User.city == "서울"  # Seoul
        sql, params = expr_korean.to_sql()
        expect(params[0]).to_equal("서울")

        # Greek
        expr_greek = User.name == "Αλέξανδρος"  # Alexandros
        sql, params = expr_greek.to_sql()
        expect(params[0]).to_equal("Αλέξανδρος")

        # Russian
        expr_russian = User.name == "Владимир"  # Vladimir
        sql, params = expr_russian.to_sql()
        expect(params[0]).to_equal("Владимир")

    def test_unicode_like_operator(self):
        """Test LIKE operator with Unicode patterns."""

        class User(Table):
            name: str
            email: str

        # Chinese pattern with wildcard
        expr_chinese = User.name.like("张%")  # Names starting with Zhang
        sql, params = expr_chinese.to_sql()
        expect("LIKE $1" in sql).to_be_true()
        expect(params[0]).to_equal("张%")

        # Arabic pattern
        expr_arabic = User.name.like("%محمد%")  # Contains Muhammad
        sql, params = expr_arabic.to_sql()
        expect(params[0]).to_equal("%محمد%")

        # Mixed Unicode and ASCII
        expr_mixed = User.email.like("%.com.%")  # Email with .com domain
        sql, params = expr_mixed.to_sql()
        expect(params[0]).to_equal("%.com.%")

        # Japanese pattern
        expr_japanese = User.name.like("田中%")  # Tanaka family names
        sql, params = expr_japanese.to_sql()
        expect(params[0]).to_equal("田中%")

    def test_unicode_in_operator(self):
        """Test IN operator with Unicode values list."""

        class User(Table):
            city: str
            status: str

        # Chinese cities
        expr_cities = User.city.in_(["北京", "上海", "广州", "深圳"])
        sql, params = expr_cities.to_sql()
        expect("IN ($1, $2, $3, $4)" in sql).to_be_true()
        expect(params).to_equal(["北京", "上海", "广州", "深圳"])

        # Arabic status values
        expr_status = User.status.in_(["نشط", "غير نشط", "محظور"])  # Active, Inactive, Banned
        sql, params = expr_status.to_sql()
        expect("IN ($1, $2, $3)" in sql).to_be_true()
        expect(params).to_equal(["نشط", "غير نشط", "محظور"])

        # Mixed scripts
        expr_mixed = User.city.in_(["Tokyo", "東京", "توكيو"])  # Tokyo in different scripts
        sql, params = expr_mixed.to_sql()
        expect(params).to_equal(["Tokyo", "東京", "توكيو"])

    def test_unicode_emoji_in_values(self):
        """Test emoji characters in filter values."""

        class Post(Table):
            title: str
            content: str
            reaction: str

        # Emoji in filter value
        expr_emoji = Post.reaction == "👍"
        sql, params = expr_emoji.to_sql()
        expect(params[0]).to_equal("👍")

        # Multiple emojis
        expr_multi = Post.title.contains("🎉🎊🎈")
        sql, params = expr_multi.to_sql()
        expect(params[0]).to_contain("🎉🎊🎈")

        # Emoji with text
        expr_combined = Post.content == "Great work! 💯🚀"
        sql, params = expr_combined.to_sql()
        expect(params[0]).to_equal("Great work! 💯🚀")

        # Skin tone modifiers
        expr_skin_tone = Post.reaction == "👋🏻"  # Waving hand with light skin tone
        sql, params = expr_skin_tone.to_sql()
        expect(params[0]).to_equal("👋🏻")

        # Family emoji (ZWJ sequence)
        expr_family = Post.content.contains("👨‍👩‍👧‍👦")  # Family emoji
        sql, params = expr_family.to_sql()
        expect(params[0]).to_contain("👨‍👩‍👧‍👦")

    def test_unicode_combining_characters(self):
        """Test combining characters (é as e + combining acute)."""

        class User(Table):
            name: str

        # Precomposed (NFC): single character é (U+00E9)
        nfc_name = "José"
        expr_nfc = User.name == nfc_name
        sql_nfc, params_nfc = expr_nfc.to_sql()
        expect(params_nfc[0]).to_equal("José")

        # Decomposed (NFD): e (U+0065) + combining acute (U+0301)
        nfd_name = "José"  # Same visual appearance, different encoding
        expr_nfd = User.name == nfd_name
        sql_nfd, params_nfd = expr_nfd.to_sql()
        # Both should be handled as-is, exact match depends on normalization
        expect(params_nfd[0]).to_equal(nfd_name)

        # Multiple combining characters
        complex_combining = "a\u0300\u0301\u0302"  # a with grave, acute, circumflex
        expr_complex = User.name == complex_combining
        sql_complex, params_complex = expr_complex.to_sql()
        expect(params_complex[0]).to_equal(complex_combining)

        # Vietnamese with combining characters
        vietnamese = "Nguyễn"  # Common Vietnamese surname
        expr_vietnamese = User.name == vietnamese
        sql_vietnamese, params_vietnamese = expr_vietnamese.to_sql()
        expect(params_vietnamese[0]).to_equal("Nguyễn")

    def test_unicode_zero_width_characters(self):
        """Test zero-width characters (ZWJ, ZWNJ)."""

        class User(Table):
            name: str
            text: str

        # Zero-width joiner (ZWJ) - used in emoji sequences
        zwj_text = "word\u200djoined"
        expr_zwj = User.name == zwj_text
        sql, params = expr_zwj.to_sql()
        expect(params[0]).to_equal(zwj_text)

        # Zero-width non-joiner (ZWNJ) - used in Persian/Arabic
        zwnj_text = "می\u200cخواهم"  # Persian: "I want"
        expr_zwnj = User.text == zwnj_text
        sql, params = expr_zwnj.to_sql()
        expect(params[0]).to_equal(zwnj_text)

        # Zero-width space (U+200B)
        zwsp_text = "word\u200bbreak"
        expr_zwsp = User.text == zwsp_text
        sql, params = expr_zwsp.to_sql()
        expect(params[0]).to_equal(zwsp_text)

        # Zero-width no-break space (ZWNBSP/BOM) (U+FEFF)
        zwnbsp_text = "\ufeffstart"
        expr_zwnbsp = User.text == zwnbsp_text
        sql, params = expr_zwnbsp.to_sql()
        expect(params[0]).to_equal(zwnbsp_text)

    def test_unicode_rtl_characters(self):
        """Test right-to-left text (Arabic, Hebrew)."""

        class User(Table):
            name: str
            address: str

        # Arabic (RTL)
        arabic_name = "محمد بن سلمان"  # Muhammad bin Salman
        expr_arabic = User.name == arabic_name
        sql, params = expr_arabic.to_sql()
        expect(params[0]).to_equal(arabic_name)

        # Hebrew (RTL)
        hebrew_name = "בנימין נתניהו"  # Benjamin Netanyahu
        expr_hebrew = User.name == hebrew_name
        sql, params = expr_hebrew.to_sql()
        expect(params[0]).to_equal(hebrew_name)

        # Mixed LTR and RTL (bidirectional text)
        mixed_text = "Hello مرحبا World"
        expr_mixed = User.address == mixed_text
        sql, params = expr_mixed.to_sql()
        expect(params[0]).to_equal(mixed_text)

        # RTL with numbers (Arabic numerals in RTL context)
        rtl_with_numbers = "الرقم ١٢٣٤٥"  # "Number 12345" in Arabic
        expr_numbers = User.address == rtl_with_numbers
        sql, params = expr_numbers.to_sql()
        expect(params[0]).to_equal(rtl_with_numbers)

        # RTL override mark (U+202E)
        rtl_override = "test\u202eoverride"
        expr_override = User.name == rtl_override
        sql, params = expr_override.to_sql()
        expect(params[0]).to_equal(rtl_override)

    def test_unicode_normalization_nfc_nfd(self):
        """Test NFC vs NFD normalization handling."""

        class User(Table):
            name: str

        # Test case 1: Single accented character
        # NFC: é as single codepoint (U+00E9)
        nfc_single = "\u00e9"  # é (precomposed)
        expr_nfc_single = User.name == nfc_single
        sql_nfc, params_nfc = expr_nfc_single.to_sql()
        expect(params_nfc[0]).to_equal("\u00e9")

        # NFD: é as base + combining (U+0065 U+0301)
        nfd_single = "\u0065\u0301"  # e + combining acute
        expr_nfd_single = User.name == nfd_single
        sql_nfd, params_nfd = expr_nfd_single.to_sql()
        expect(params_nfd[0]).to_equal("\u0065\u0301")

        # Test case 2: Full name with multiple accents
        # NFC form
        nfc_name = "Zürich"  # Precomposed ü
        expr_nfc_name = User.name == nfc_name
        sql, params = expr_nfc_name.to_sql()
        expect(params[0]).to_equal("Zürich")

        # Test case 3: Korean - decomposes significantly
        # NFC: 한 (U+D55C) - single character
        nfc_korean = "\ud55c"
        # NFD: ㅎ + ㅏ + ㄴ (U+1112 U+1161 U+11AB) - three characters
        nfd_korean = "\u1112\u1161\u11ab"

        expr_nfc_korean = User.name == nfc_korean
        sql_k1, params_k1 = expr_nfc_korean.to_sql()
        expect(params_k1[0]).to_equal(nfc_korean)

        expr_nfd_korean = User.name == nfd_korean
        sql_k2, params_k2 = expr_nfd_korean.to_sql()
        expect(params_k2[0]).to_equal(nfd_korean)

        # Test case 4: Complex combining sequence
        # ḱ̃ (k + acute + tilde) - multiple combining marks
        complex_nfd = "k\u0301\u0303"
        expr_complex = User.name == complex_nfd
        sql_complex, params_complex = expr_complex.to_sql()
        expect(params_complex[0]).to_equal(complex_nfd)

        # Test case 5: LIKE pattern with NFD
        # Pattern should work regardless of normalization
        pattern_nfd = "e\u0301%"  # é% in NFD form
        expr_pattern = User.name.like(pattern_nfd)
        sql_pattern, params_pattern = expr_pattern.to_sql()
        expect(params_pattern[0]).to_equal(pattern_nfd)
