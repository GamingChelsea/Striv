import { useState } from 'react'
import { Check, Copy } from 'lucide-react'
import { CodeLang, getTokenClasses, tokenizeLine } from '../lib/highlight'
import { DialectMode, ThemeMode } from '../types/docs'

interface CodeBlockProps {
  code: string
  odinCode?: string
  lang?: CodeLang
  filename?: string
  dialect?: DialectMode
  theme?: ThemeMode
  showLineNumbers?: boolean
}

export function CodeBlock({
  code,
  odinCode,
  lang = 'striv',
  filename,
  dialect = 'rust',
  theme = 'light',
  showLineNumbers = true,
}: CodeBlockProps) {
  const [copied, setCopied] = useState(false)
  const activeCode = lang === 'striv' && dialect === 'odin' && odinCode ? odinCode : code
  const lines = activeCode.replace(/\n$/, '').split('\n')
  const tokenClasses = getTokenClasses(theme)
  const isDark = theme === 'dark'

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(activeCode.trimEnd())
      setCopied(true)
      setTimeout(() => setCopied(false), 1600)
    } catch {
      // ignore clipboard errors
    }
  }

  return (
    <div
      className={`rounded-md border overflow-hidden my-4 ${
        isDark
          ? 'border-zinc-800 bg-[#12151a] text-zinc-200'
          : 'border-zinc-200 bg-zinc-50 text-zinc-800'
      }`}
    >
      <div
        className={`flex items-center justify-between px-3.5 py-2 border-b text-xs ${
          isDark
            ? 'border-zinc-800 bg-zinc-900/70 text-zinc-400'
            : 'border-zinc-200 bg-zinc-100/80 text-zinc-600'
        }`}
      >
        <div className="flex items-center gap-2 min-w-0">
          <span className="font-mono text-[11px] uppercase tracking-wider text-amber-600 font-semibold">
            {lang}
          </span>
          {lang === 'striv' && odinCode && (
            <span
              className={`font-mono text-[10px] px-1.5 py-0.5 rounded border ${
                isDark
                  ? 'border-zinc-700 bg-zinc-800 text-zinc-300'
                  : 'border-zinc-300 bg-white text-zinc-700'
              }`}
            >
              {dialect === 'odin' ? 'Odin-Stil' : 'Rust-Stil'}
            </span>
          )}
          {filename && (
            <>
              <span className="opacity-40">/</span>
              <span className="font-mono truncate">{filename}</span>
            </>
          )}
        </div>

        <button
          type="button"
          onClick={handleCopy}
          aria-label="Code kopieren"
          className={`inline-flex items-center gap-1.5 px-2 py-0.5 text-[11px] font-mono rounded border transition-colors cursor-pointer ${
            isDark
              ? 'border-transparent hover:border-zinc-700 hover:text-zinc-100'
              : 'border-transparent hover:border-zinc-300 hover:text-zinc-900'
          }`}
        >
          {copied ? (
            <>
              <Check className="w-3 h-3 text-emerald-500" />
              <span className="text-emerald-500">Kopiert</span>
            </>
          ) : (
            <>
              <Copy className="w-3 h-3" />
              <span>Kopieren</span>
            </>
          )}
        </button>
      </div>

      <div className="code-scroll overflow-x-auto font-mono text-[13px] leading-6 py-3">
        <table className="w-full border-collapse">
          <tbody>
            {lines.map((line, idx) => {
              const tokens = tokenizeLine(line, lang)
              return (
                <tr key={idx}>
                  {showLineNumbers && (
                    <td
                      className={`select-none text-right pl-3.5 pr-3 w-10 align-top text-[11px] ${
                        isDark ? 'text-zinc-600' : 'text-zinc-400'
                      }`}
                    >
                      {idx + 1}
                    </td>
                  )}
                  <td className={`pr-4 whitespace-pre ${showLineNumbers ? '' : 'pl-4'}`}>
                    {tokens.length === 0
                      ? '\u00A0'
                      : tokens.map((tok, tIdx) => (
                          <span key={tIdx} className={tokenClasses[tok.kind]}>
                            {tok.text}
                          </span>
                        ))}
                  </td>
                </tr>
              )
            })}
          </tbody>
        </table>
      </div>
    </div>
  )
}
