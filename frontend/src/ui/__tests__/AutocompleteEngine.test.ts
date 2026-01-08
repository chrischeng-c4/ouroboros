import { describe, it, expect } from 'vitest';
import { AutocompleteEngine } from '../AutocompleteEngine';

describe('AutocompleteEngine', () => {
  const engine = new AutocompleteEngine();

  describe('Function suggestions', () => {
    it('suggests functions after = sign', () => {
      const context = engine.createContext('=S', 2);
      const suggestions = engine.getSuggestions(context);

      expect(suggestions.length).toBeGreaterThan(0);
      expect(suggestions[0].type).toBe('function');
      expect(suggestions[0].value).toMatch(/^S/);
    });

    it('filters functions by prefix', () => {
      const context = engine.createContext('=SUM', 4);
      const suggestions = engine.getSuggestions(context);
      const values = suggestions.map(s => s.value);

      expect(values).toContain('SUM');
      expect(values.every(v => v.startsWith('SUM') || v.startsWith('S'))).toBe(true);
    });

    it('returns empty for non-formula context', () => {
      const context = engine.createContext('Plain text', 5);
      const suggestions = engine.getSuggestions(context);

      expect(suggestions).toHaveLength(0);
    });
  });

  describe('Cell reference suggestions', () => {
    it('suggests cell references for partial column', () => {
      const context = engine.createContext('=A', 2);
      const suggestions = engine.getSuggestions(context);
      const cellRefs = suggestions.filter(s => s.type === 'cell-reference');

      expect(cellRefs.length).toBeGreaterThan(0);
      expect(cellRefs[0].value).toMatch(/^A\d+/);
    });

    it('suggests cell reference for complete pattern', () => {
      const context = engine.createContext('=A1', 3);
      const suggestions = engine.getSuggestions(context);
      const cellRefs = suggestions.filter(s => s.type === 'cell-reference');

      expect(cellRefs.length).toBeGreaterThan(0);
    });
  });

  describe('Context parsing', () => {
    it('detects formula context', () => {
      expect(engine.isFormulaContext('=SUM(A1)')).toBe(true);
      expect(engine.isFormulaContext('Plain')).toBe(false);
    });

    it('extracts current token', () => {
      const token = engine.getCurrentToken('=SUM(A', 6);
      expect(token.text).toBe('A');
      expect(token.startPos).toBe(5);
    });

    it('creates context correctly', () => {
      const ctx = engine.createContext('=SUM', 4);
      expect(ctx.fullText).toBe('=SUM');
      expect(ctx.cursorPosition).toBe(4);
      expect(ctx.isFormula).toBe(true);
      expect(ctx.currentToken).toBe('SUM');
    });
  });
});
