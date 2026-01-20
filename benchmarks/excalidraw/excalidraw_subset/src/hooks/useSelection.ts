import { useState, useCallback } from 'react';
import { Selection, Element } from '../types';
import { getBoundingBox } from '../utils/geometry';

export function useSelection() {
  const [selection, setSelection] = useState<Selection>({
    elements: [],
    bounds: null,
  });

  const select = useCallback((elements: Element[]) => {
    const points = elements.flatMap((el) => [
      el.position,
      { x: el.position.x + el.size.width, y: el.position.y + el.size.height },
    ]);
    const bounds = getBoundingBox(points);

    setSelection({
      elements,
      bounds,
    });
  }, []);

  const clearSelection = useCallback(() => {
    setSelection({
      elements: [],
      bounds: null,
    });
  }, []);

  const addToSelection = useCallback((element: Element) => {
    setSelection((prev) => {
      const elements = [...prev.elements, element];
      const points = elements.flatMap((el) => [
        el.position,
        { x: el.position.x + el.size.width, y: el.position.y + el.size.height },
      ]);
      const bounds = getBoundingBox(points);

      return {
        elements,
        bounds,
      };
    });
  }, []);

  return {
    selection,
    select,
    clearSelection,
    addToSelection,
  };
}
