import { Point, Size } from '../types';

export function distance(p1: Point, p2: Point): number {
  const dx = p2.x - p1.x;
  const dy = p2.y - p1.y;
  return Math.sqrt(dx * dx + dy * dy);
}

export function midpoint(p1: Point, p2: Point): Point {
  return {
    x: (p1.x + p2.x) / 2,
    y: (p1.y + p2.y) / 2,
  };
}

export function rotate(point: Point, center: Point, angle: number): Point {
  const cos = Math.cos(angle);
  const sin = Math.sin(angle);
  const dx = point.x - center.x;
  const dy = point.y - center.y;

  return {
    x: center.x + dx * cos - dy * sin,
    y: center.y + dx * sin + dy * cos,
  };
}

export function getBoundingBox(points: Point[]): { min: Point; max: Point } | null {
  if (points.length === 0) return null;

  const xs = points.map((p) => p.x);
  const ys = points.map((p) => p.y);

  return {
    min: { x: Math.min(...xs), y: Math.min(...ys) },
    max: { x: Math.max(...xs), y: Math.max(...ys) },
  };
}

export function isPointInRect(point: Point, rectPos: Point, rectSize: Size): boolean {
  return (
    point.x >= rectPos.x &&
    point.x <= rectPos.x + rectSize.width &&
    point.y >= rectPos.y &&
    point.y <= rectPos.y + rectSize.height
  );
}

export function scaleSize(size: Size, factor: number): Size {
  return {
    width: size.width * factor,
    height: size.height * factor,
  };
}
