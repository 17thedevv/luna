// TextMate grammar definition for Luna Programming Language
export const luna = {
  name: 'luna',
  id: 'luna',
  scopeName: 'source.luna',
  displayName: 'Luna',
  aliases: ['ln', 'mellis', 'ms'],
  patterns: [
    // Line comments
    {
      match: /\/\/.*$/,
      name: 'comment.line.double-slash.luna',
    },
    // Block comments
    {
      begin: /\/\*/,
      end: /\*\//,
      name: 'comment.block.luna',
    },
    // Raw strings r#"..."# or r"..."
    {
      begin: /r#"/,
      end: /"#/,
      name: 'string.quoted.raw.luna',
    },
    // Double quoted strings
    {
      begin: /"/,
      end: /"/,
      name: 'string.quoted.double.luna',
      patterns: [
        { match: /\\./, name: 'constant.character.escape.luna' },
        { match: /\{[0-9a-zA-Z_:]*\}/, name: 'variable.interpolation.luna' },
      ],
    },
    // Character literals
    {
      match: /b?'(?:\\.|[^\\'])'/,
      name: 'constant.character.luna',
    },
    // Byte string literals
    {
      match: /b"[^"]*"/,
      name: 'string.quoted.byte.luna',
    },
    // Numbers (Hex, Binary, Octal, Floats, Integers)
    {
      match: /\b0x[0-9a-fA-F_]+\b/,
      name: 'constant.numeric.hex.luna',
    },
    {
      match: /\b0b[01_]+\b/,
      name: 'constant.numeric.binary.luna',
    },
    {
      match: /\b0o[0-7_]+\b/,
      name: 'constant.numeric.octal.luna',
    },
    {
      match: /\b\d[\d_]*\.\d[\d_]*(?:[eE][+-]?\d+)?(?:f32|f64)?\b/,
      name: 'constant.numeric.float.luna',
    },
    {
      match: /\b\d[\d_]*(?:i8|i16|i32|i64|i128|u8|u16|u32|u64|u128|isize|usize)?\b/,
      name: 'constant.numeric.integer.luna',
    },
    // Attributes / Annotations @[attr]
    {
      match: /@\[[a-zA-Z_][a-zA-Z0-9_]*(?:\(.*?\))?\]/,
      name: 'meta.annotation.luna entity.other.attribute-name.luna',
    },
    // Core Keywords
    {
      match: /\b(dec|const|rw|fn|struct|enum|trait|impl|export|import|using|as|unsafe|comptime|extern|requires|anchor|life|life_from|where|outlives|type)\b/,
      name: 'keyword.control.luna',
    },
    // Control Flow
    {
      match: /\b(return|if|else|loop|while|for|in|break|continue|match)\b/,
      name: 'keyword.control.flow.luna',
    },
    // Async
    {
      match: /\b(async)\b/,
      name: 'keyword.control.async.luna',
    },
    {
      match: /\.await\b/,
      name: 'keyword.control.await.luna',
    },
    // Boolean & Constants
    {
      match: /\b(true|false|null)\b/,
      name: 'constant.language.luna',
    },
    // Self identifier
    {
      match: /\b(self|Self)\b/,
      name: 'variable.language.self.luna',
    },
    // Built-in Primitive Types
    {
      match: /\b(i8|i16|i32|i64|i128|isize|u8|u16|u32|u64|u128|usize|int_32|uint_32|f32|f64|bool|char|str|void)\b/,
      name: 'support.type.primitive.luna',
    },
    // Standard Library Common Types
    {
      match: /\b(Option|Result|Some|None|Ok|Err|Vec|String|Box|HashMap|HashSet)\b/,
      name: 'support.type.luna',
    },
    // Reference and pointer operators: &rw, &T, *rw, *T
    {
      match: /&(?:rw)?|\*(?:rw)?/,
      name: 'keyword.operator.reference.luna',
    },
    // Match arrow -> and Lambda pipe |x|
    {
      match: /->|=>/,
      name: 'keyword.operator.arrow.luna',
    },
    // Namespace separator ::
    {
      match: /::/,
      name: 'punctuation.separator.namespace.luna',
    },
    // Function calls
    {
      match: /\b([a-z_][a-z0-9_]*)\s*(?=\()/,
      name: 'entity.name.function.luna',
    },
    // Type names (PascalCase)
    {
      match: /\b[A-Z][a-zA-Z0-9_]*\b/,
      name: 'entity.name.type.luna',
    },
    // Operators
    {
      match: /[+\-*/%=!<>|&^~]+/,
      name: 'keyword.operator.luna',
    },
    // Macro placeholder @identifier
    {
      match: /@[a-zA-Z_][a-zA-Z0-9_]*/,
      name: 'variable.other.metavariable.luna',
    },
  ],
};

export default luna;
