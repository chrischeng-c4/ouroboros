#!/usr/bin/env python
"""
Verification script for PyLoop Phase 5: Middleware & Production Features

Run this script to verify that Phase 5 is fully implemented and working.
"""

import sys


def verify_imports():
    """Verify all new classes can be imported."""
    print("=" * 70)
    print("PHASE 5 VERIFICATION")
    print("=" * 70)
    print("\n[1/5] Checking imports...")

    try:
        from data_bridge.pyloop import (
            App,
            BaseMiddleware,
            CORSMiddleware,
            LoggingMiddleware,
            CompressionMiddleware,
            HTTPException
        )
        print("  ✓ All middleware classes imported successfully")
        return True
    except ImportError as e:
        print(f"  ✗ Import failed: {e}")
        return False


def verify_base_middleware():
    """Verify BaseMiddleware is abstract."""
    print("\n[2/5] Checking BaseMiddleware...")

    try:
        from data_bridge.pyloop import BaseMiddleware

        # Should not be able to instantiate abstract class
        try:
            BaseMiddleware()
            print("  ✗ BaseMiddleware is not abstract")
            return False
        except TypeError:
            print("  ✓ BaseMiddleware is abstract (as expected)")
            return True
    except Exception as e:
        print(f"  ✗ Error: {e}")
        return False


def verify_cors_middleware():
    """Verify CORSMiddleware works."""
    print("\n[3/5] Checking CORSMiddleware...")

    try:
        from data_bridge.pyloop import CORSMiddleware

        # Create with defaults
        cors1 = CORSMiddleware()
        assert cors1.allow_all_origins is True
        assert cors1.max_age == 600
        print("  ✓ Default configuration works")

        # Create with custom settings
        cors2 = CORSMiddleware(
            allow_origins=["https://example.com"],
            allow_methods=["GET", "POST"],
            allow_credentials=True
        )
        assert cors2.allow_all_origins is False
        assert "https://example.com" in cors2.allow_origins
        assert cors2.allow_credentials is True
        print("  ✓ Custom configuration works")

        # Test origin checking
        assert cors2._is_origin_allowed("https://example.com") is True
        assert cors2._is_origin_allowed("https://evil.com") is False
        print("  ✓ Origin checking works")

        return True
    except Exception as e:
        print(f"  ✗ Error: {e}")
        return False


def verify_app_integration():
    """Verify App middleware integration."""
    print("\n[4/5] Checking App integration...")

    try:
        from data_bridge.pyloop import App, CORSMiddleware, LoggingMiddleware

        app = App()
        assert hasattr(app, 'middlewares')
        assert len(app.middlewares) == 0
        print("  ✓ App has middleware list")

        # Add middleware
        cors = CORSMiddleware()
        logging_mid = LoggingMiddleware()

        app.add_middleware(cors)
        app.add_middleware(logging_mid)

        assert len(app.middlewares) == 2
        assert app.middlewares[0] is cors
        assert app.middlewares[1] is logging_mid
        print("  ✓ Middleware registration works")

        # Verify middleware methods exist
        assert hasattr(app, '_process_middleware_request')
        assert hasattr(app, '_process_middleware_response')
        assert hasattr(app, '_wrap_handler_with_middleware')
        print("  ✓ Middleware processing methods exist")

        return True
    except Exception as e:
        print(f"  ✗ Error: {e}")
        return False


def verify_example():
    """Verify example loads correctly."""
    print("\n[5/5] Checking example...")

    try:
        sys.path.insert(0, 'examples')
        import pyloop_middleware_example

        app = pyloop_middleware_example.app
        assert len(app.middlewares) == 4
        print(f"  ✓ Example loaded with {len(app.middlewares)} middleware")

        middleware_names = [m.__class__.__name__ for m in app.middlewares]
        expected = ["CORSMiddleware", "LoggingMiddleware", "RateLimitMiddleware", "AuthMiddleware"]

        for name in expected:
            if name in middleware_names:
                print(f"    ✓ {name}")
            else:
                print(f"    ✗ {name} missing")
                return False

        return True
    except Exception as e:
        print(f"  ✗ Error: {e}")
        return False


def main():
    """Run all verifications."""
    checks = [
        verify_imports,
        verify_base_middleware,
        verify_cors_middleware,
        verify_app_integration,
        verify_example
    ]

    results = [check() for check in checks]

    print("\n" + "=" * 70)
    print("VERIFICATION RESULTS")
    print("=" * 70)
    print(f"\nPassed: {sum(results)}/{len(results)}")

    if all(results):
        print("\n✅ Phase 5 implementation is COMPLETE and working correctly!\n")
        print("Next steps:")
        print("  1. Run tests: python -m pytest tests/test_pyloop_middleware*.py -v")
        print("  2. Try example: python examples/pyloop_middleware_example.py")
        print("  3. Read summary: cat PYLOOP_PHASE5_SUMMARY.md")
        return 0
    else:
        print("\n❌ Some checks failed. Please review the errors above.\n")
        return 1


if __name__ == "__main__":
    sys.exit(main())
