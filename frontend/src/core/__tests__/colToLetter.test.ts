import { describe, it, expect } from 'vitest';

/**
 * Convert column index to Excel-style letter notation
 * (Copied from main.ts for testing)
 */
function colToLetter(col: number): string {
  let result = '';
  let num = col;
  while (num >= 0) {
    result = String.fromCharCode(65 + (num % 26)) + result;
    num = Math.floor(num / 26) - 1;
    if (num < 0) break;
  }
  return result;
}

describe('colToLetter (Bug #8 verification)', () => {
  it('converts single letters correctly', () => {
    expect(colToLetter(0)).toBe('A');
    expect(colToLetter(1)).toBe('B');
    expect(colToLetter(25)).toBe('Z');
  });

  it('converts double letters correctly', () => {
    expect(colToLetter(26)).toBe('AA');
    expect(colToLetter(27)).toBe('AB');
    expect(colToLetter(51)).toBe('AZ');
    expect(colToLetter(52)).toBe('BA');
  });

  it('converts triple letters correctly', () => {
    expect(colToLetter(702)).toBe('AAA');
  });

  it('handles typical spreadsheet range', () => {
    expect(colToLetter(0)).toBe('A');
    expect(colToLetter(9)).toBe('J');
    expect(colToLetter(99)).toBe('CV');
  });
});
