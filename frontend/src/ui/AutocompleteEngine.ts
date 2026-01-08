export interface Suggestion {
  type: 'function' | 'cell-reference' | 'range';
  value: string;           // "SUM", "A1", "A1:B10"
  display: string;         // "SUM()" with formatting
  description?: string;    // "Adds numbers together"
  insertText: string;      // What gets inserted
  cursorOffset?: number;   // Where cursor goes after insert
}

export interface AutocompleteContext {
  fullText: string;
  cursorPosition: number;
  isFormula: boolean;
  currentToken: string;
  tokenStartPos: number;
}

// Function registry
const FUNCTIONS = [
  { name: 'SUM', syntax: 'SUM(number1, number2, ...)', description: 'Adds all numbers', category: 'Math' },
  { name: 'AVERAGE', syntax: 'AVERAGE(number1, ...)', description: 'Returns the average', category: 'Statistical' },
  { name: 'COUNT', syntax: 'COUNT(value1, ...)', description: 'Counts numbers', category: 'Statistical' },
  { name: 'MAX', syntax: 'MAX(number1, ...)', description: 'Returns the maximum value', category: 'Statistical' },
  { name: 'MIN', syntax: 'MIN(number1, ...)', description: 'Returns the minimum value', category: 'Statistical' },
  { name: 'IF', syntax: 'IF(condition, true_value, false_value)', description: 'Conditional logic', category: 'Logical' },
  { name: 'ABS', syntax: 'ABS(number)', description: 'Absolute value', category: 'Math' },
  { name: 'ROUND', syntax: 'ROUND(number, decimals)', description: 'Rounds a number', category: 'Math' },
];

export class AutocompleteEngine {
  /**
   * Get suggestions based on current input context
   */
  public getSuggestions(context: AutocompleteContext): Suggestion[] {
    if (!context.isFormula) {
      return []; // Don't suggest in non-formula context
    }

    const suggestions: Suggestion[] = [];
    const token = context.currentToken.toUpperCase();

    // Suggest functions
    if (token.length > 0) {
      for (const fn of FUNCTIONS) {
        if (fn.name.startsWith(token)) {
          suggestions.push({
            type: 'function',
            value: fn.name,
            display: fn.syntax,
            description: fn.description,
            insertText: `${fn.name}(`,
            cursorOffset: 0, // Cursor stays after opening paren
          });
        }
      }
    }

    // Suggest cell references (A1, B2, etc.)
    if (this.looksLikeCellRef(context.currentToken)) {
      const cellSuggestions = this.generateCellReferences(context.currentToken);
      suggestions.push(...cellSuggestions);
    }

    return suggestions.slice(0, 10); // Limit to 10 suggestions
  }

  /**
   * Check if current context is a formula (starts with =)
   */
  public isFormulaContext(text: string): boolean {
    return text.trim().startsWith('=');
  }

  /**
   * Extract current token and its position
   */
  public getCurrentToken(text: string, cursorPos: number): { text: string; startPos: number } {
    // Find the start of the current token
    let start = cursorPos - 1;
    while (start >= 0 && /[A-Za-z0-9]/.test(text[start])) {
      start--;
    }
    start++; // Move to first character of token

    const tokenText = text.substring(start, cursorPos);
    return { text: tokenText, startPos: start };
  }

  /**
   * Check if text looks like a cell reference pattern
   */
  private looksLikeCellRef(text: string): boolean {
    return /^[A-Z]+[0-9]*$/.test(text.toUpperCase());
  }

  /**
   * Generate cell reference suggestions based on partial input
   */
  private generateCellReferences(partial: string): Suggestion[] {
    const suggestions: Suggestion[] = [];
    const upper = partial.toUpperCase();

    // If just letters (like "A"), suggest A1-A10
    if (/^[A-Z]+$/.test(upper)) {
      for (let i = 1; i <= 10; i++) {
        suggestions.push({
          type: 'cell-reference',
          value: `${upper}${i}`,
          display: `${upper}${i}`,
          insertText: `${upper}${i}`,
        });
      }
    }
    // If letters + partial number (like "A1"), suggest completions
    else if (/^[A-Z]+[0-9]+$/.test(upper)) {
      suggestions.push({
        type: 'cell-reference',
        value: upper,
        display: upper,
        insertText: upper,
      });
    }

    return suggestions;
  }

  /**
   * Create autocomplete context from text and cursor position
   */
  public createContext(text: string, cursorPos: number): AutocompleteContext {
    const token = this.getCurrentToken(text, cursorPos);
    return {
      fullText: text,
      cursorPosition: cursorPos,
      isFormula: this.isFormulaContext(text),
      currentToken: token.text,
      tokenStartPos: token.startPos,
    };
  }
}
