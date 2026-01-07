"""
Health check endpoints for K8s probes.

- /health - Basic health check (always returns 200 if app is running)
- /live - Liveness probe (app is alive and not deadlocked)
- /ready - Readiness probe (app is ready to accept traffic)
"""
from typing import Callable, Dict, Any, Optional, List
from dataclasses import dataclass, field
import asyncio


@dataclass
class HealthCheck:
    """Individual health check"""
    name: str
    check: Callable[[], bool]  # Returns True if healthy
    critical: bool = True  # If critical, failure means not ready


@dataclass
class HealthStatus:
    """Health check result"""
    status: str  # "healthy", "degraded", "unhealthy"
    checks: Dict[str, bool] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "status": self.status,
            "checks": self.checks
        }


class HealthManager:
    """Manages health checks for K8s probes"""

    def __init__(self):
        self._checks: List[HealthCheck] = []
        self._is_ready: bool = False

    def add_check(self, name: str, check: Callable, critical: bool = True) -> None:
        """Add a health check"""
        self._checks.append(HealthCheck(name=name, check=check, critical=critical))

    def set_ready(self, ready: bool = True) -> None:
        """Manually set readiness state"""
        self._is_ready = ready

    async def check_health(self) -> HealthStatus:
        """Run all health checks"""
        results = {}
        all_healthy = True
        critical_healthy = True

        for hc in self._checks:
            try:
                if asyncio.iscoroutinefunction(hc.check):
                    healthy = await hc.check()
                else:
                    healthy = hc.check()
                results[hc.name] = healthy
                if not healthy:
                    all_healthy = False
                    if hc.critical:
                        critical_healthy = False
            except Exception:
                results[hc.name] = False
                all_healthy = False
                if hc.critical:
                    critical_healthy = False

        if all_healthy:
            status = "healthy"
        elif critical_healthy:
            status = "degraded"
        else:
            status = "unhealthy"

        return HealthStatus(status=status, checks=results)

    def is_live(self) -> bool:
        """Check if app is alive (not deadlocked)"""
        return True  # Basic: if we can respond, we're alive

    async def is_ready(self) -> bool:
        """Check if app is ready to accept traffic"""
        if not self._is_ready:
            return False
        health = await self.check_health()
        return health.status != "unhealthy"
