from __future__ import annotations

from collections.abc import Callable

from ..base.fields import Field


class TTLField(Field[int]):
    """Field for Redis TTL (Time To Live) values in seconds."""

    def __init__(
        self,
        *,
        default: int | None = None,
        default_factory: Callable[[], int] | None = None,
        required: bool = False,
        db_field: str | None = None,
    ) -> None:
        super().__init__(
            default=default,
            default_factory=default_factory,
            required=required,
            db_field=db_field,
        )
