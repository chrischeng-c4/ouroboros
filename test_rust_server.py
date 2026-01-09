"""
Test script for Rust server integration.

This script tests the new app.run() integration with the Rust HTTP server.
"""
from data_bridge.api import App

app = App(title="Test API", version="1.0.0")


@app.get("/")
async def root():
    """Root endpoint."""
    return {"message": "Hello from Rust server!"}


@app.get("/health")
async def health():
    """Health check endpoint."""
    return {"status": "healthy", "server": "Rust HTTP Server"}


@app.post("/echo")
async def echo(body: dict):
    """Echo endpoint - returns the request body."""
    return {"echo": body}


if __name__ == "__main__":
    import sys

    # Parse command line args
    use_rust = "--asgi" not in sys.argv

    if use_rust:
        print("Testing with Rust HTTP server (high performance)")
        print("Visit: http://127.0.0.1:8000")
        print("Try: curl http://127.0.0.1:8000/")
        print("Try: curl http://127.0.0.1:8000/health")
        print("Try: curl -X POST http://127.0.0.1:8000/echo -H 'Content-Type: application/json' -d '{\"test\":\"data\"}'")
        app.run(host="127.0.0.1", port=8000, use_rust_server=True)
    else:
        print("Testing with ASGI/uvicorn (fallback)")
        print("Note: This is slower than the Rust server")
        app.run(host="127.0.0.1", port=8000, use_rust_server=False)
