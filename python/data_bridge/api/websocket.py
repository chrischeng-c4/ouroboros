"""
WebSocket support for API handlers.
"""

from typing import Any, Optional
from enum import IntEnum


class WebSocketState(IntEnum):
    """WebSocket connection states."""

    CONNECTING = 0
    CONNECTED = 1
    DISCONNECTED = 2


class WebSocketDisconnect(Exception):
    """Exception raised when WebSocket connection is closed."""

    def __init__(self, code: int = 1000, reason: str = ""):
        """Initialize WebSocketDisconnect exception.

        Args:
            code: WebSocket close code (default: 1000 - normal closure)
            reason: Optional close reason string
        """
        self.code = code
        self.reason = reason
        super().__init__(f"WebSocket disconnected: code={code}, reason={reason!r}")

    def __repr__(self) -> str:
        return f"WebSocketDisconnect(code={self.code}, reason={self.reason!r})"


class WebSocket:
    """Represents an active WebSocket connection.

    Provides async methods for sending and receiving messages over a WebSocket
    connection, with support for text, JSON, and binary data.

    Example:
        @app.websocket("/ws")
        async def websocket_endpoint(websocket: WebSocket):
            await websocket.accept()
            try:
                while True:
                    data = await websocket.receive_json()
                    await websocket.send_json({"echo": data})
            except WebSocketDisconnect:
                print("Client disconnected")
    """

    def __init__(self, connection: Any):
        """Initialize WebSocket with the raw connection.

        Args:
            connection: The underlying WebSocket connection object
        """
        self._connection = connection
        self._state = WebSocketState.CONNECTING

    @property
    def client(self) -> tuple[str, int]:
        """Get client address as (host, port) tuple.

        Returns:
            Tuple of (host, port) for the connected client

        Raises:
            NotImplementedError: Rust backend not yet implemented
        """
        raise NotImplementedError(
            "WebSocket.client is not yet implemented - Rust backend pending"
        )

    @property
    def state(self) -> WebSocketState:
        """Get current connection state.

        Returns:
            Current WebSocketState (CONNECTING, CONNECTED, or DISCONNECTED)
        """
        return self._state

    async def accept(self, subprotocol: Optional[str] = None) -> None:
        """Accept the WebSocket connection.

        This must be called before any send/receive operations.

        Args:
            subprotocol: Optional WebSocket subprotocol to use

        Raises:
            NotImplementedError: Rust backend not yet implemented
        """
        del subprotocol  # Unused in stub - will be used when Rust backend is implemented
        raise NotImplementedError(
            "WebSocket.accept() is not yet implemented - Rust backend pending"
        )

    async def send_text(self, data: str) -> None:
        """Send a text message over the WebSocket.

        Args:
            data: Text string to send

        Raises:
            WebSocketDisconnect: If connection is closed
            NotImplementedError: Rust backend not yet implemented
        """
        del data  # Unused in stub - will be used when Rust backend is implemented
        raise NotImplementedError(
            "WebSocket.send_text() is not yet implemented - Rust backend pending"
        )

    async def receive_text(self) -> str:
        """Receive a text message from the WebSocket.

        Returns:
            Text string received from the client

        Raises:
            WebSocketDisconnect: If connection is closed
            NotImplementedError: Rust backend not yet implemented
        """
        raise NotImplementedError(
            "WebSocket.receive_text() is not yet implemented - Rust backend pending"
        )

    async def send_json(self, data: Any) -> None:
        """Send JSON data over the WebSocket.

        The data will be serialized to JSON and sent as a text message.

        Args:
            data: Any JSON-serializable Python object

        Raises:
            WebSocketDisconnect: If connection is closed
            TypeError: If data is not JSON-serializable
            NotImplementedError: Rust backend not yet implemented
        """
        del data  # Unused in stub - will be used when Rust backend is implemented
        # Once implemented, this would do:
        # json_str = json.dumps(data)
        # await self.send_text(json_str)
        raise NotImplementedError(
            "WebSocket.send_json() is not yet implemented - Rust backend pending"
        )

    async def receive_json(self) -> Any:
        """Receive and parse JSON data from the WebSocket.

        Returns:
            Parsed Python object from JSON data

        Raises:
            WebSocketDisconnect: If connection is closed
            json.JSONDecodeError: If received data is not valid JSON
            NotImplementedError: Rust backend not yet implemented
        """
        # Once implemented, this would do:
        # text = await self.receive_text()
        # return json.loads(text)
        raise NotImplementedError(
            "WebSocket.receive_json() is not yet implemented - Rust backend pending"
        )

    async def send_bytes(self, data: bytes) -> None:
        """Send binary data over the WebSocket.

        Args:
            data: Binary data to send

        Raises:
            WebSocketDisconnect: If connection is closed
            NotImplementedError: Rust backend not yet implemented
        """
        del data  # Unused in stub - will be used when Rust backend is implemented
        raise NotImplementedError(
            "WebSocket.send_bytes() is not yet implemented - Rust backend pending"
        )

    async def receive_bytes(self) -> bytes:
        """Receive binary data from the WebSocket.

        Returns:
            Binary data received from the client

        Raises:
            WebSocketDisconnect: If connection is closed
            NotImplementedError: Rust backend not yet implemented
        """
        raise NotImplementedError(
            "WebSocket.receive_bytes() is not yet implemented - Rust backend pending"
        )

    async def close(self, code: int = 1000, reason: str = "") -> None:
        """Close the WebSocket connection.

        Args:
            code: WebSocket close code (default: 1000 - normal closure)
            reason: Optional close reason string

        Common close codes:
            1000 - Normal closure
            1001 - Going away
            1002 - Protocol error
            1003 - Unsupported data
            1007 - Invalid frame payload data
            1008 - Policy violation
            1009 - Message too big
            1010 - Mandatory extension
            1011 - Internal server error

        Raises:
            NotImplementedError: Rust backend not yet implemented
        """
        del code, reason  # Unused in stub - will be used when Rust backend is implemented
        raise NotImplementedError(
            "WebSocket.close() is not yet implemented - Rust backend pending"
        )
