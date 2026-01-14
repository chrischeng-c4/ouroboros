## ADDED Requirements

### Requirement: Generate New ObjectId
The system SHALL provide a method to generate a new, unique ObjectId.

#### Scenario: Generate ID
- **WHEN** `ObjectId.new()` is called
- **THEN** a new valid ObjectId is returned
- **AND** it is unique (statistically)

### Requirement: Validate ObjectId String
The system SHALL provide a method to validate if a string is a valid ObjectId hex representation (24 hex characters).

#### Scenario: Valid Hex String
- **WHEN** `ObjectId.is_valid("507f1f77bcf86cd799439011")` is called
- **THEN** return `True`

#### Scenario: Invalid Hex String (Length)
- **WHEN** `ObjectId.is_valid("123")` is called
- **THEN** return `False`

#### Scenario: Invalid Hex String (Chars)
- **WHEN** `ObjectId.is_valid("507f1f77bcf86cd7994390zz")` is called
- **THEN** return `False`

### Requirement: Parse ObjectId from String
The system SHALL allow creating an ObjectId from a 24-character hex string.

#### Scenario: From Valid String
- **WHEN** `ObjectId("507f1f77bcf86cd799439011")` or `ObjectId.from_str("507f1f77bcf86cd799439011")` is called
- **THEN** an ObjectId instance is returned
- **AND** its string representation matches the input

#### Scenario: From Invalid String
- **WHEN** `ObjectId("bad")` is called
- **THEN** raise `ValueError` (or `InvalidId`)

### Requirement: String Representation
The ObjectId SHALL provide a string representation matching the 24-character hex code.

#### Scenario: String conversion
- **WHEN** `str(oid)` is called
- **THEN** return the 24-char hex string

#### Scenario: Repr conversion
- **WHEN** `repr(oid)` is called
- **THEN** return a string in the format `ObjectId('507f1f77bcf86cd799439011')`

### Requirement: Equality Comparison
The ObjectId SHALL support equality comparison with other ObjectIds and string representations.

#### Scenario: Compare Equal ObjectIds
- **WHEN** two `ObjectId` instances with the same value are compared (`==`)
- **THEN** return `True`

#### Scenario: Compare Different ObjectIds
- **WHEN** two `ObjectId` instances with different values are compared
- **THEN** return `False`

#### Scenario: Compare with String
- **WHEN** an `ObjectId` is compared with its valid hex string (`oid == "hex_str"`)
- **THEN** return `False` (Strict typing: ObjectId is not a string, consistent with bson.ObjectId)
