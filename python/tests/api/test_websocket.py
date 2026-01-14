"""
Tests for WebSocket support.
"""

import pytest
from ouroboros.api import WebSocket, WebSocketState, WebSocketDisconnect, App


class TestWebSocketState:
    """Test WebSocketState enum."""

    def test_state_values(self):
        """Test enum values match expected integers."""
        assert WebSocketState.CONNECTING == 0
        assert WebSocketState.CONNECTED == 1
        assert WebSocketState.DISCONNECTED == 2

    def test_state_is_int_enum(self):
        """Test that WebSocketState is an IntEnum."""
        assert isinstance(WebSocketState.CONNECTING, int)
        assert isinstance(WebSocketState.CONNECTED, int)
        assert isinstance(WebSocketState.DISCONNECTED, int)

    def test_state_comparison(self):
        """Test state comparison operations."""
        assert WebSocketState.CONNECTING < WebSocketState.CONNECTED
        assert WebSocketState.CONNECTED < WebSocketState.DISCONNECTED
        assert WebSocketState.DISCONNECTED > WebSocketState.CONNECTING

    def test_state_string_representation(self):
        """Test string representation of states."""
        assert str(WebSocketState.CONNECTING) == "0"
        assert str(WebSocketState.CONNECTED) == "1"
        assert str(WebSocketState.DISCONNECTED) == "2"


class TestWebSocketDisconnect:
    """Test WebSocketDisconnect exception."""

    def test_default_values(self):
        """Test exception with default values."""
        exc = WebSocketDisconnect()
        assert exc.code == 1000
        assert exc.reason == ""

    def test_custom_code(self):
        """Test exception with custom code."""
        exc = WebSocketDisconnect(code=1001)
        assert exc.code == 1001
        assert exc.reason == ""

    def test_custom_reason(self):
        """Test exception with custom reason."""
        exc = WebSocketDisconnect(reason="Client closed connection")
        assert exc.code == 1000
        assert exc.reason == "Client closed connection"

    def test_custom_code_and_reason(self):
        """Test exception with both custom code and reason."""
        exc = WebSocketDisconnect(code=1008, reason="Policy violation")
        assert exc.code == 1008
        assert exc.reason == "Policy violation"

    def test_string_representation(self):
        """Test string representation of exception."""
        exc = WebSocketDisconnect(code=1000, reason="Normal closure")
        expected = "WebSocket disconnected: code=1000, reason='Normal closure'"
        assert str(exc) == expected

    def test_repr_representation(self):
        """Test repr representation of exception."""
        exc = WebSocketDisconnect(code=1001, reason="Going away")
        expected = "WebSocketDisconnect(code=1001, reason='Going away')"
        assert repr(exc) == expected

    def test_repr_with_empty_reason(self):
        """Test repr with empty reason."""
        exc = WebSocketDisconnect(code=1000)
        expected = "WebSocketDisconnect(code=1000, reason='')"
        assert repr(exc) == expected

    def test_is_exception(self):
        """Test that WebSocketDisconnect is an Exception."""
        exc = WebSocketDisconnect()
        assert isinstance(exc, Exception)

    def test_raise_and_catch(self):
        """Test raising and catching the exception."""
        with pytest.raises(WebSocketDisconnect) as exc_info:
            raise WebSocketDisconnect(code=1002, reason="Protocol error")

        assert exc_info.value.code == 1002
        assert exc_info.value.reason == "Protocol error"


class TestWebSocket:
    """Test WebSocket class."""

    def test_initial_state(self):
        """Test initial state is CONNECTING."""
        ws = WebSocket(connection=object())
        assert ws.state == WebSocketState.CONNECTING

    def test_initial_state_value(self):
        """Test initial state value is 0."""
        ws = WebSocket(connection=object())
        assert ws.state == 0

    def test_connection_storage(self):
        """Test connection is stored internally."""
        connection = object()
        ws = WebSocket(connection=connection)
        assert ws._connection is connection

    def test_client_property_not_implemented(self):
        """Test client property raises NotImplementedError."""
        ws = WebSocket(connection=object())
        with pytest.raises(NotImplementedError) as exc_info:
            _ = ws.client

        assert "WebSocket.client is not yet implemented" in str(exc_info.value)
        assert "Rust backend pending" in str(exc_info.value)

    @pytest.mark.asyncio
    async def test_accept_not_implemented(self):
        """Test accept() raises NotImplementedError."""
        ws = WebSocket(connection=object())
        with pytest.raises(NotImplementedError) as exc_info:
            await ws.accept()

        assert "WebSocket.accept() is not yet implemented" in str(exc_info.value)
        assert "Rust backend pending" in str(exc_info.value)

    @pytest.mark.asyncio
    async def test_accept_with_subprotocol_not_implemented(self):
        """Test accept() with subprotocol raises NotImplementedError."""
        ws = WebSocket(connection=object())
        with pytest.raises(NotImplementedError) as exc_info:
            await ws.accept(subprotocol="chat")

        assert "WebSocket.accept() is not yet implemented" in str(exc_info.value)

    @pytest.mark.asyncio
    async def test_send_text_not_implemented(self):
        """Test send_text() raises NotImplementedError."""
        ws = WebSocket(connection=object())
        with pytest.raises(NotImplementedError) as exc_info:
            await ws.send_text("Hello")

        assert "WebSocket.send_text() is not yet implemented" in str(exc_info.value)
        assert "Rust backend pending" in str(exc_info.value)

    @pytest.mark.asyncio
    async def test_receive_text_not_implemented(self):
        """Test receive_text() raises NotImplementedError."""
        ws = WebSocket(connection=object())
        with pytest.raises(NotImplementedError) as exc_info:
            await ws.receive_text()

        assert "WebSocket.receive_text() is not yet implemented" in str(exc_info.value)
        assert "Rust backend pending" in str(exc_info.value)

    @pytest.mark.asyncio
    async def test_send_json_not_implemented(self):
        """Test send_json() raises NotImplementedError."""
        ws = WebSocket(connection=object())
        with pytest.raises(NotImplementedError) as exc_info:
            await ws.send_json({"key": "value"})

        assert "WebSocket.send_json() is not yet implemented" in str(exc_info.value)
        assert "Rust backend pending" in str(exc_info.value)

    @pytest.mark.asyncio
    async def test_receive_json_not_implemented(self):
        """Test receive_json() raises NotImplementedError."""
        ws = WebSocket(connection=object())
        with pytest.raises(NotImplementedError) as exc_info:
            await ws.receive_json()

        assert "WebSocket.receive_json() is not yet implemented" in str(exc_info.value)
        assert "Rust backend pending" in str(exc_info.value)

    @pytest.mark.asyncio
    async def test_send_bytes_not_implemented(self):
        """Test send_bytes() raises NotImplementedError."""
        ws = WebSocket(connection=object())
        with pytest.raises(NotImplementedError) as exc_info:
            await ws.send_bytes(b"binary data")

        assert "WebSocket.send_bytes() is not yet implemented" in str(exc_info.value)
        assert "Rust backend pending" in str(exc_info.value)

    @pytest.mark.asyncio
    async def test_receive_bytes_not_implemented(self):
        """Test receive_bytes() raises NotImplementedError."""
        ws = WebSocket(connection=object())
        with pytest.raises(NotImplementedError) as exc_info:
            await ws.receive_bytes()

        assert "WebSocket.receive_bytes() is not yet implemented" in str(exc_info.value)
        assert "Rust backend pending" in str(exc_info.value)

    @pytest.mark.asyncio
    async def test_close_not_implemented(self):
        """Test close() raises NotImplementedError."""
        ws = WebSocket(connection=object())
        with pytest.raises(NotImplementedError) as exc_info:
            await ws.close()

        assert "WebSocket.close() is not yet implemented" in str(exc_info.value)
        assert "Rust backend pending" in str(exc_info.value)

    @pytest.mark.asyncio
    async def test_close_with_code_not_implemented(self):
        """Test close() with custom code raises NotImplementedError."""
        ws = WebSocket(connection=object())
        with pytest.raises(NotImplementedError) as exc_info:
            await ws.close(code=1001)

        assert "WebSocket.close() is not yet implemented" in str(exc_info.value)

    @pytest.mark.asyncio
    async def test_close_with_code_and_reason_not_implemented(self):
        """Test close() with code and reason raises NotImplementedError."""
        ws = WebSocket(connection=object())
        with pytest.raises(NotImplementedError) as exc_info:
            await ws.close(code=1000, reason="Normal closure")

        assert "WebSocket.close() is not yet implemented" in str(exc_info.value)


class TestAppWebSocketDecorator:
    """Test App websocket decorator."""

    def test_decorator_registers_handler(self):
        """Test @app.websocket decorator registers handler."""
        app = App()

        @app.websocket("/ws")
        async def websocket_handler(websocket: WebSocket):
            pass

        assert "/ws" in app._websocket_handlers
        assert app._websocket_handlers["/ws"] is websocket_handler

    def test_decorator_with_path_parameters(self):
        """Test decorator with path parameters."""
        app = App()

        @app.websocket("/ws/{room_id}")
        async def websocket_handler(websocket: WebSocket, room_id: str):
            pass

        assert "/ws/{room_id}" in app._websocket_handlers
        assert app._websocket_handlers["/ws/{room_id}"] is websocket_handler

    def test_decorator_with_multiple_parameters(self):
        """Test decorator with multiple path parameters."""
        app = App()

        @app.websocket("/ws/{room_id}/{user_id}")
        async def websocket_handler(websocket: WebSocket, room_id: str, user_id: str):
            pass

        assert "/ws/{room_id}/{user_id}" in app._websocket_handlers

    def test_decorator_returns_original_function(self):
        """Test decorator returns original function unchanged."""
        app = App()

        async def websocket_handler(websocket: WebSocket):
            pass

        decorated = app.websocket("/ws")(websocket_handler)

        assert decorated is websocket_handler

    def test_decorator_with_name(self):
        """Test decorator with custom name."""
        app = App()

        @app.websocket("/ws", name="custom_websocket")
        async def websocket_handler(websocket: WebSocket):
            pass

        assert "/ws" in app._websocket_handlers

    def test_decorator_with_summary(self):
        """Test decorator with summary."""
        app = App()

        @app.websocket("/ws", summary="WebSocket endpoint")
        async def websocket_handler(websocket: WebSocket):
            pass

        assert "/ws" in app._websocket_handlers

    def test_decorator_with_description(self):
        """Test decorator with description."""
        app = App()

        @app.websocket("/ws", description="Handles WebSocket connections")
        async def websocket_handler(websocket: WebSocket):
            pass

        assert "/ws" in app._websocket_handlers

    def test_decorator_with_tags(self):
        """Test decorator with tags."""
        app = App()

        @app.websocket("/ws", tags=["websocket", "realtime"])
        async def websocket_handler(websocket: WebSocket):
            pass

        assert "/ws" in app._websocket_handlers

    def test_decorator_with_deprecated(self):
        """Test decorator with deprecated flag."""
        app = App()

        @app.websocket("/ws", deprecated=True)
        async def websocket_handler(websocket: WebSocket):
            pass

        assert "/ws" in app._websocket_handlers

    def test_decorator_with_all_parameters(self):
        """Test decorator with all optional parameters."""
        app = App()

        @app.websocket(
            "/ws",
            name="my_websocket",
            summary="WebSocket endpoint",
            description="Handles WebSocket connections",
            tags=["websocket"],
            deprecated=False
        )
        async def websocket_handler(websocket: WebSocket):
            pass

        assert "/ws" in app._websocket_handlers

    def test_multiple_websocket_handlers(self):
        """Test registering multiple WebSocket handlers."""
        app = App()

        @app.websocket("/ws/chat")
        async def chat_handler(websocket: WebSocket):
            pass

        @app.websocket("/ws/notifications")
        async def notification_handler(websocket: WebSocket):
            pass

        assert "/ws/chat" in app._websocket_handlers
        assert "/ws/notifications" in app._websocket_handlers
        assert app._websocket_handlers["/ws/chat"] is chat_handler
        assert app._websocket_handlers["/ws/notifications"] is notification_handler

    def test_websocket_handler_not_called_during_decoration(self):
        """Test that handler function is not called during decoration."""
        app = App()
        called = False

        @app.websocket("/ws")
        async def websocket_handler(websocket: WebSocket):
            nonlocal called
            called = True

        assert not called
        assert "/ws" in app._websocket_handlers


if __name__ == "__main__":
    # Run with: uv run python tests/api/test_websocket.py
    pytest.main([__file__, "-v"])
