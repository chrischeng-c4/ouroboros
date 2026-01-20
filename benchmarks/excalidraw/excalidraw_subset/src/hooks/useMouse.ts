import { useState, useCallback, RefObject } from 'react';
import { Point } from '../types';

export function useMouse(canvasRef: RefObject<HTMLCanvasElement>) {
  const [position, setPosition] = useState<Point>({ x: 0, y: 0 });
  const [isDown, setIsDown] = useState(false);

  const handleMouseMove = useCallback(
    (event: MouseEvent) => {
      if (!canvasRef.current) return;

      const rect = canvasRef.current.getBoundingClientRect();
      setPosition({
        x: event.clientX - rect.left,
        y: event.clientY - rect.top,
      });
    },
    [canvasRef]
  );

  const handleMouseDown = useCallback(() => {
    setIsDown(true);
  }, []);

  const handleMouseUp = useCallback(() => {
    setIsDown(false);
  }, []);

  return {
    position,
    isDown,
    handleMouseMove,
    handleMouseDown,
    handleMouseUp,
  };
}
