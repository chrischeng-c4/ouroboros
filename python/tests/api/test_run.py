"""Tests for app.run() method."""
import pytest
from data_bridge.test import expect
from unittest.mock import Mock, patch
from data_bridge.api import App


class TestAppRun:
    """Test app.run() functionality."""

    def test_run_method_exists(self):
        """Test that run method exists."""
        app = App()
        assert hasattr(app, "run")
        assert callable(app.run)

    @patch("uvicorn.run")
    def test_run_calls_uvicorn(self, mock_uvicorn_run):
        """Test that run() calls uvicorn.run()."""
        app = App()
        app.run(host="0.0.0.0", port=3000)

        mock_uvicorn_run.assert_called_once()
        call_args = mock_uvicorn_run.call_args
        assert call_args[0][0] == app  # First arg is the app
        assert call_args[1]["host"] == "0.0.0.0"
        assert call_args[1]["port"] == 3000

    @patch("uvicorn.run")
    def test_run_default_values(self, mock_uvicorn_run):
        """Test default values for run()."""
        app = App()
        app.run()

        call_args = mock_uvicorn_run.call_args[1]
        assert call_args["host"] == "127.0.0.1"
        assert call_args["port"] == 8000
        assert call_args["reload"] is False
        assert call_args["workers"] == 1
        assert call_args["log_level"] == "info"
        assert call_args["access_log"] is True

    @patch("uvicorn.run")
    def test_run_custom_config(self, mock_uvicorn_run):
        """Test custom configuration."""
        app = App()
        app.run(
            host="localhost",
            port=5000,
            reload=True,
            workers=4,
            log_level="debug",
            access_log=False
        )

        call_args = mock_uvicorn_run.call_args[1]
        assert call_args["host"] == "localhost"
        assert call_args["port"] == 5000
        assert call_args["reload"] is True
        assert call_args["workers"] == 4
        assert call_args["log_level"] == "debug"
        assert call_args["access_log"] is False

    @patch("uvicorn.run")
    def test_run_extra_kwargs(self, mock_uvicorn_run):
        """Test passing extra kwargs to uvicorn."""
        app = App()
        app.run(ssl_keyfile="/path/to/key", ssl_certfile="/path/to/cert")

        call_args = mock_uvicorn_run.call_args[1]
        assert call_args["ssl_keyfile"] == "/path/to/key"
        assert call_args["ssl_certfile"] == "/path/to/cert"

    def test_run_without_uvicorn_raises_error(self):
        """Test that missing uvicorn raises helpful error."""
        app = App()

        with patch.dict("sys.modules", {"uvicorn": None}):
            expect(lambda: app.run()).to_raise(ImportError)
