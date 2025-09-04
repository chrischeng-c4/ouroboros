from __future__ import annotations

from typing import TYPE_CHECKING, ClassVar, TypeVar

from ...base.model import BaseModel

if TYPE_CHECKING:
    from .backend import MongoSyncBackend
    from .collection import MongoCollection

T = TypeVar("T", bound="Document")


class Document(BaseModel):
    """MongoDB Document model for synchronous operations."""

    _collection: ClassVar[str]
    _database: ClassVar[str | None]
    _backend: ClassVar[MongoSyncBackend | None] = None

    @classmethod
    def objects(cls: type[T]) -> MongoCollection[T]:
        """Return a collection manager instance for this document."""
        from .collection import MongoCollection
        return MongoCollection(cls)

    @classmethod
    def set_backend(cls, backend: MongoSyncBackend) -> None:
        """Set the MongoDB backend for this document."""
        cls._backend = backend

    def save(self) -> None:
        """Save the document instance to MongoDB."""
        if not self._backend:
            raise RuntimeError(f"No backend configured for {self.__class__.__name__}")
        self._backend.save(self)

    def delete(self) -> None:
        """Delete the document instance from MongoDB."""
        if not self._backend:
            raise RuntimeError(f"No backend configured for {self.__class__.__name__}")
        self._backend.delete(self)
