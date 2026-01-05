"""Form data and file upload support.

This module provides FastAPI-compatible Form and File dependencies
for handling multipart/form-data and application/x-www-form-urlencoded requests.
"""

from typing import Any, Optional
from dataclasses import dataclass


@dataclass
class UploadFile:
    """Uploaded file from multipart form.

    This class provides an interface similar to FastAPI's UploadFile,
    allowing access to uploaded file data and metadata.

    Attributes:
        filename: Original filename from the upload
        content_type: MIME type of the uploaded file
        data: File contents as bytes
        field_name: Name of the form field

    Example:
        @app.post("/upload")
        async def upload_file(file: UploadFile = File(...)):
            content = await file.read()
            return {
                "filename": file.filename,
                "size": file.size,
                "content_type": file.content_type
            }
    """
    filename: str
    content_type: str
    data: bytes
    field_name: str = ""

    async def read(self) -> bytes:
        """Read file contents.

        Returns:
            File data as bytes

        Example:
            content = await file.read()
            print(f"Read {len(content)} bytes")
        """
        return self.data

    async def write(self, data: bytes) -> int:
        """Write data to file (in-memory only).

        Args:
            data: Bytes to write

        Returns:
            Number of bytes written

        Example:
            await file.write(b"new content")
        """
        self.data = data
        return len(data)

    async def seek(self, offset: int) -> int:
        """Seek to position (not implemented for in-memory files).

        Args:
            offset: Position to seek to

        Returns:
            Always returns 0 (in-memory files don't support seeking)
        """
        return 0

    async def close(self) -> None:
        """Close file (no-op for in-memory files).

        In-memory files don't need explicit closing, but this method
        is provided for API compatibility.
        """
        pass

    @property
    def size(self) -> int:
        """Get file size in bytes.

        Returns:
            Size of file data

        Example:
            if file.size > 10_000_000:
                raise ValueError("File too large")
        """
        return len(self.data)


class FormMarker:
    """Marker for form field dependency.

    This class is used internally to mark parameters that should be
    extracted from form data. Use the Form() function instead of
    instantiating this directly.

    Attributes:
        default: Default value if field not provided (...  means required)
    """
    def __init__(self, default: Any = ...):
        self.default = default


class FileMarker:
    """Marker for file upload dependency.

    This class is used internally to mark parameters that should be
    extracted from file uploads. Use the File() function instead of
    instantiating this directly.

    Attributes:
        default: Default value if file not provided (... means required)
    """
    def __init__(self, default: Any = ...):
        self.default = default


def Form(default: Any = ...) -> Any:
    """Mark parameter as form field (FastAPI compatible).

    Use this to extract text fields from multipart/form-data or
    application/x-www-form-urlencoded requests.

    Args:
        default: Default value if field not provided. Use ... to make required.

    Returns:
        FormMarker instance for internal use

    Example:
        @app.post("/submit")
        async def submit_form(
            name: str = Form(...),
            email: str = Form(...),
            age: int = Form(None)
        ):
            return {
                "name": name,
                "email": email,
                "age": age
            }
    """
    return FormMarker(default)


def File(default: Any = ...) -> Any:
    """Mark parameter as file upload (FastAPI compatible).

    Use this to extract uploaded files from multipart/form-data requests.

    Args:
        default: Default value if file not provided. Use ... to make required.

    Returns:
        FileMarker instance for internal use

    Example:
        @app.post("/upload")
        async def upload_file(
            file: UploadFile = File(...),
            description: str = Form(None)
        ):
            data = await file.read()
            return {
                "filename": file.filename,
                "size": len(data),
                "description": description
            }

        # Multiple files
        @app.post("/upload-multiple")
        async def upload_files(
            files: list[UploadFile] = File(...)
        ):
            return {
                "count": len(files),
                "filenames": [f.filename for f in files]
            }
    """
    return FileMarker(default)
