import { ThemeMode } from '../types/docs'

export type CodeLang = 'striv' | 'rust' | 'bash' | 'toml' | 'csharp'

export type TokenKind =
  | 'plain'
  | 'comment'
  | 'string'
  | 'number'
  | 'keyword'
  | 'type'
  | 'operator'
  | 'function'

export interface Token {
  kind: TokenKind
  text: string
}

const STRIV_KEYWORDS = new Set([
  'fn',
  'proc',
  'struct',
  'impl',
  'enum',
  'open',
  'extend',
  'let',
  'mut',
  'const',
  'tweak',
  'use',
  'import',
  'if',
  'else',
  'for',
  'in',
  'while',
  'match',
  'return',
  'break',
  'continue',
  'defer',
  'yield',
  'block',
  'unit',
  'test',
  'assert',
  'true',
  'false',
  'self',
  'shared',
  'lock',
  'concurrent',
  'spawn',
  'parallel',
  'query',
  'select',
  'timeout',
  'rpc',
  'sync',
  'rust',
  'extern',
  'foreign',
  'Ok',
  'Err',
  'Some',
  'None',
  'null',
  'print',
])

const STRIV_TYPES = new Set([
  'int',
  'float',
  'f32',
  'u32',
  'i32',
  'u8',
  'i8',
  'bool',
  'String',
  'cstr',
  'Vec',
  'Deque',
  'Map',
  'StrMap',
  'BTree',
  'Set',
  'Heap',
  'Grid',
  'Bytes',
  'Rng',
  'BitSet',
  'Channel',
  'Atomic',
  'Ecs',
  'Vec3',
  'Mat4',
  'Result',
  'Option',
])

const RUST_KEYWORDS = new Set([
  'fn',
  'let',
  'mut',
  'const',
  'struct',
  'enum',
  'impl',
  'pub',
  'use',
  'import',
  'if',
  'else',
  'for',
  'in',
  'while',
  'match',
  'return',
  'move',
  'true',
  'false',
  'self',
  'unsafe',
])

function tokenizeCodeLine(line: string, kwSet: Set<string>, tySet: Set<string>): Token[] {
  const tokens: Token[] = []
  let i = 0

  while (i < line.length) {
    if (line[i] === '/' && line[i + 1] === '/') {
      tokens.push({ kind: 'comment', text: line.slice(i) })
      break
    }

    if (line[i] === '"') {
      let j = i + 1
      while (j < line.length) {
        if (line[j] === '\\' && j + 1 < line.length) {
          j += 2
        } else if (line[j] === '"') {
          j++
          break
        } else {
          j++
        }
      }
      tokens.push({ kind: 'string', text: line.slice(i, j) })
      i = j
      continue
    }

    if (/[0-9]/.test(line[i])) {
      let j = i
      if (line[j] === '0' && (line[j + 1] === 'x' || line[j + 1] === 'b')) {
        j += 2
        while (j < line.length && /[0-9a-fA-F_]/.test(line[j])) j++
      } else {
        while (j < line.length && /[0-9_.]/.test(line[j])) {
          if (line[j] === '.' && line[j + 1] === '.') break
          j++
        }
      }
      if (line[j] === '_' && /[a-zA-Z]/.test(line[j + 1] ?? '')) {
        j++
        while (j < line.length && /[a-zA-Z0-9_]/.test(line[j])) j++
      }
      tokens.push({ kind: 'number', text: line.slice(i, j) })
      i = j
      continue
    }

    if (/[a-zA-Z_]/.test(line[i])) {
      let j = i + 1
      while (j < line.length && /[a-zA-Z0-9_]/.test(line[j])) j++
      const word = line.slice(i, j)

      if (kwSet.has(word)) {
        tokens.push({ kind: 'keyword', text: word })
      } else if (tySet.has(word) || /^[A-Z][A-Za-z0-9_]*$/.test(word)) {
        tokens.push({ kind: 'type', text: word })
      } else if (line[j] === '(') {
        tokens.push({ kind: 'function', text: word })
      } else {
        tokens.push({ kind: 'plain', text: word })
      }
      i = j
      continue
    }

    const two = line.slice(i, i + 2)
    if (['->', '=>', '::', ':=', '..', '<<', '>>', '&&', '||', '==', '!=', '<=', '>='].includes(two)) {
      tokens.push({ kind: 'operator', text: two })
      i += 2
      continue
    }

    if ('!?:+-*/=<>|&^'.includes(line[i])) {
      tokens.push({ kind: 'operator', text: line[i] })
      i++
      continue
    }

    let j = i + 1
    while (j < line.length && !/[a-zA-Z0-9_"/!?:+\-*/=<>|&^]/.test(line[j])) {
      j++
    }
    tokens.push({ kind: 'plain', text: line.slice(i, j) })
    i = j
  }

  return tokens
}

function tokenizeBashLine(line: string): Token[] {
  if (line.trimStart().startsWith('#')) {
    return [{ kind: 'comment', text: line }]
  }
  const parts = line.split(/(\s+)/)
  let seenCmd = false
  const tokens: Token[] = []
  for (const part of parts) {
    if (!part) continue
    if (/^\s+$/.test(part)) {
      tokens.push({ kind: 'plain', text: part })
    } else if (!seenCmd && (part === 'striv' || part === 'cargo' || part === 'cd')) {
      tokens.push({ kind: 'keyword', text: part })
      seenCmd = true
    } else if (part.startsWith('--') || part.startsWith('-')) {
      tokens.push({ kind: 'type', text: part })
    } else if (part.startsWith('"') || part.startsWith("'")) {
      tokens.push({ kind: 'string', text: part })
    } else if (
      seenCmd &&
      ['new', 'run', 'dev', 'live', 'test', 'fmt', 'build', 'sync', 'bindgen', 'bindgen-cs', 'switch-syntax', 'lsp'].includes(part)
    ) {
      tokens.push({ kind: 'function', text: part })
    } else {
      tokens.push({ kind: 'plain', text: part })
      seenCmd = true
    }
  }
  return tokens
}

function tokenizeTomlLine(line: string): Token[] {
  if (line.trimStart().startsWith('#')) return [{ kind: 'comment', text: line }]
  if (line.trimStart().startsWith('[')) return [{ kind: 'keyword', text: line }]
  const eq = line.indexOf('=')
  if (eq === -1) return [{ kind: 'plain', text: line }]
  return [
    { kind: 'type', text: line.slice(0, eq) },
    { kind: 'operator', text: '=' },
    { kind: 'string', text: line.slice(eq + 1) },
  ]
}

export function tokenizeLine(line: string, lang: CodeLang): Token[] {
  if (lang === 'striv') return tokenizeCodeLine(line, STRIV_KEYWORDS, STRIV_TYPES)
  if (lang === 'rust' || lang === 'csharp') return tokenizeCodeLine(line, RUST_KEYWORDS, STRIV_TYPES)
  if (lang === 'bash') return tokenizeBashLine(line)
  if (lang === 'toml') return tokenizeTomlLine(line)
  return [{ kind: 'plain', text: line }]
}

export const TOKEN_CLASSES_DARK: Record<TokenKind, string> = {
  plain: 'text-zinc-200',
  comment: 'text-zinc-500 italic',
  string: 'text-emerald-400',
  number: 'text-amber-300',
  keyword: 'text-amber-400 font-medium',
  type: 'text-sky-300',
  operator: 'text-zinc-400',
  function: 'text-zinc-100 font-medium',
}

export const TOKEN_CLASSES_LIGHT: Record<TokenKind, string> = {
  plain: 'text-zinc-800',
  comment: 'text-zinc-500 italic',
  string: 'text-emerald-700',
  number: 'text-amber-700',
  keyword: 'text-amber-700 font-semibold',
  type: 'text-sky-700',
  operator: 'text-zinc-500',
  function: 'text-zinc-900 font-medium',
}

export function getTokenClasses(theme: ThemeMode): Record<TokenKind, string> {
  return theme === 'dark' ? TOKEN_CLASSES_DARK : TOKEN_CLASSES_LIGHT
}
