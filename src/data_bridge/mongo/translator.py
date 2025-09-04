from __future__ import annotations

from typing import Any, ClassVar

from ..base.fields import CompoundExpression, QueryExpression


class MongoQueryTranslator:
    """Translates query expressions to MongoDB query format."""

    OPERATOR_MAPPING: ClassVar[dict[str, str | None]] = {
        "eq": None,  # MongoDB uses implicit equality
        "ne": "$ne",
        "gt": "$gt",
        "gte": "$gte",
        "lt": "$lt",
        "lte": "$lte",
        "in": "$in",
        "nin": "$nin",
        "regex": "$regex",
        "exists": "$exists",
        "size": "$size",
    }

    @classmethod
    def translate(cls, expressions: list[QueryExpression | CompoundExpression]) -> dict[str, Any]:
        """Translate a list of expressions to MongoDB query."""
        if not expressions:
            return {}

        if len(expressions) == 1:
            return cls._translate_single(expressions[0])

        # Multiple expressions are implicitly ANDed
        translated_expressions = [cls._translate_single(expr) for expr in expressions]
        return {"$and": translated_expressions}

    @classmethod
    def _translate_single(cls, expression: QueryExpression | CompoundExpression) -> dict[str, Any]:
        """Translate a single expression."""
        if isinstance(expression, QueryExpression):
            return cls._translate_query_expression(expression)
        elif isinstance(expression, CompoundExpression):
            return cls._translate_compound_expression(expression)
        else:
            raise ValueError(f"Unknown expression type: {type(expression)}")

    @classmethod
    def _translate_query_expression(cls, expr: QueryExpression) -> dict[str, Any]:
        """Translate a query expression to MongoDB format."""
        field = expr.field
        operator = expr.operator
        value = expr.value

        if operator == "eq":
            # Simple equality
            return {field: value}
        elif operator in cls.OPERATOR_MAPPING:
            mongo_op = cls.OPERATOR_MAPPING[operator]
            return {field: {mongo_op: value}}
        else:
            raise ValueError(f"Unsupported operator: {operator}")

    @classmethod
    def _translate_compound_expression(cls, expr: CompoundExpression) -> dict[str, Any]:
        """Translate a compound expression to MongoDB format."""
        operator = expr.operator
        expressions = expr.expressions

        if operator == "and":
            translated_expressions = [cls._translate_single(op) for op in expressions]
            return {"$and": translated_expressions}
        elif operator == "or":
            translated_expressions = [cls._translate_single(op) for op in expressions]
            return {"$or": translated_expressions}
        elif operator == "not":
            if len(expressions) != 1:
                raise ValueError("NOT operator must have exactly one operand")
            translated_expression = cls._translate_single(expressions[0])
            return {"$not": translated_expression}
        else:
            raise ValueError(f"Unsupported compound operator: {operator}")

    @classmethod
    def translate_sort(cls, sort_fields: list[tuple[str, int]]) -> list[tuple[str, int]]:
        """Translate sort specifications to MongoDB format."""
        # MongoDB uses the same format as our internal representation
        return sort_fields

    @classmethod
    def translate_projection(cls, fields: list[str]) -> dict[str, int]:
        """Translate field projection to MongoDB format."""
        if not fields:
            return {}

        projection = {}
        for field in fields:
            projection[field] = 1
        return projection
