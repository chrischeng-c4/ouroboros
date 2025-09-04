from __future__ import annotations

from typing import TYPE_CHECKING, ClassVar, TypeVar

from ...base.model import BaseModel

if TYPE_CHECKING:
    from .backend import MongoAsyncBackend
    from .collection import AsyncMongoCollection

T = TypeVar("T", bound="AsyncDocument")


class AsyncDocument(BaseModel):
    """MongoDB Document model for asynchronous operations."""

    _collection: ClassVar[str]
    _database: ClassVar[str | None]
    _backend: ClassVar[MongoAsyncBackend | None] = None

    @classmethod
    def objects(cls: type[T]) -> AsyncMongoCollection[T]:
        """Return a collection manager instance for this document."""
        from .collection import AsyncMongoCollection
        return AsyncMongoCollection(cls)

    @classmethod
    def set_backend(cls, backend: MongoAsyncBackend) -> None:
        """Set the MongoDB backend for this document."""
        cls._backend = backend

    async def save(self) -> None:
        """Save the document instance to MongoDB."""
        if not self._backend:
            raise RuntimeError(f"No backend configured for {self.__class__.__name__}")
        await self._backend.save(self)

    async def delete(self) -> None:
        """Delete the document instance from MongoDB."""
        if not self._backend:
            raise RuntimeError(f"No backend configured for {self.__class__.__name__}")
        await self._backend.delete(self)
