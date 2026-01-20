import { Color } from '../types';

export function hexToRgb(hex: string): { r: number; g: number; b: number } | null {
  const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  return result
    ? {
        r: parseInt(result[1], 16),
        g: parseInt(result[2], 16),
        b: parseInt(result[3], 16),
      }
    : null;
}

export function rgbToHex(r: number, g: number, b: number): string {
  return '#' + [r, g, b].map((x) => {
    const hex = x.toString(16);
    return hex.length === 1 ? '0' + hex : hex;
  }).join('');
}

export function colorToString(color: Color): string {
  const { r, g, b } = color.rgb;
  return `rgba(${r}, ${g}, ${b}, ${color.alpha})`;
}

export function parseColor(colorString: string): Color | null {
  const rgb = hexToRgb(colorString);
  if (!rgb) return null;

  return {
    hex: colorString,
    rgb,
    alpha: 1.0,
  };
}

export function interpolateColor(color1: Color, color2: Color, t: number): Color {
  const r = Math.round(color1.rgb.r + (color2.rgb.r - color1.rgb.r) * t);
  const g = Math.round(color1.rgb.g + (color2.rgb.g - color1.rgb.g) * t);
  const b = Math.round(color1.rgb.b + (color2.rgb.b - color1.rgb.b) * t);
  const alpha = color1.alpha + (color2.alpha - color1.alpha) * t;

  return {
    hex: rgbToHex(r, g, b),
    rgb: { r, g, b },
    alpha,
  };
}

export const PRESET_COLORS = [
  '#000000',
  '#FFFFFF',
  '#FF0000',
  '#00FF00',
  '#0000FF',
  '#FFFF00',
  '#FF00FF',
  '#00FFFF',
];
