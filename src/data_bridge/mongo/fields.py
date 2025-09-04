from __future__ import annotations

from collections.abc import Callable

from bson import ObjectId

from ..base.fields import Field


class ObjectIdField(Field[ObjectId]):
    """Field for MongoDB ObjectId values."""

    def __init__(
        self,
        *,
        primary_key: bool = False,
        default: ObjectId | None = None,
        default_factory: Callable[[], ObjectId] | None = None,
        required: bool = False,
        db_field: str | None = None,
    ) -> None:
        super().__init__(
            default=default,
            default_factory=default_factory,
            required=required,
            primary_key=primary_key,
            db_field=db_field,
        )
