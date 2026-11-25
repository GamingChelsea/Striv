import { ReactNode } from 'react'
import { DialectMode, LangMode, ThemeMode } from '../types/docs'

export interface ChapterProps {
  dialect: DialectMode
  theme: ThemeMode
  lang: LangMode
}

export function DocH1({ children }: { children: ReactNode }) {
  return <h1 className="text-3xl font-semibold tracking-tight">{children}</h1>
}

export function DocH2({
  id,
  theme,
  children,
}: {
  id: string
  theme: ThemeMode
  children: ReactNode
}) {
  return (
    <h2
      id={id}
      className={`scroll-mt-20 text-xl font-semibold tracking-tight pt-4 border-t ${
        theme === 'dark' ? 'border-zinc-800/80' : 'border-zinc-200/80'
      }`}
    >
      {children}
    </h2>
  )
}

export function DocH3({ children }: { children: ReactNode }) {
  return <h3 className="text-base font-semibold tracking-tight pt-2">{children}</h3>
}

export function DocP({
  theme,
  children,
}: {
  theme: ThemeMode
  children: ReactNode
}) {
  return (
    <p className={`text-sm leading-7 ${theme === 'dark' ? 'text-zinc-300' : 'text-zinc-700'}`}>
      {children}
    </p>
  )
}

export function InlineCode({
  theme,
  children,
}: {
  theme: ThemeMode
  children: ReactNode
}) {
  return (
    <code
      className={`font-mono text-[12.5px] px-1.5 py-0.5 rounded border ${
        theme === 'dark'
          ? 'border-zinc-800 bg-zinc-900 text-amber-300'
          : 'border-zinc-200 bg-zinc-100 text-zinc-900'
      }`}
    >
      {children}
    </code>
  )
}

export function DocTable({
  theme,
  headers,
  rows,
}: {
  theme: ThemeMode
  headers: string[]
  rows: (string | ReactNode)[][]
}) {
  const isDark = theme === 'dark'
  return (
    <div
      className={`overflow-x-auto rounded-md border my-4 ${
        isDark ? 'border-zinc-800 bg-[#12151a]' : 'border-zinc-200 bg-white'
      }`}
    >
      <table className="w-full text-left border-collapse text-xs">
        <thead>
          <tr
            className={`border-b font-mono ${
              isDark
                ? 'border-zinc-800 bg-zinc-900/70 text-zinc-400'
                : 'border-zinc-200 bg-zinc-50 text-zinc-600'
            }`}
          >
            {headers.map((h) => (
              <th key={h} className="py-2.5 px-4 font-medium">
                {h}
              </th>
            ))}
          </tr>
        </thead>
        <tbody className={`divide-y ${isDark ? 'divide-zinc-800/80' : 'divide-zinc-200/80'}`}>
          {rows.map((row, idx) => (
            <tr key={idx} className={isDark ? 'hover:bg-zinc-900/40' : 'hover:bg-zinc-50'}>
              {row.map((cell, cIdx) => (
                <td
                  key={cIdx}
                  className={`py-2.5 px-4 align-top ${
                    cIdx === 0
                      ? `font-mono font-medium ${isDark ? 'text-zinc-100' : 'text-zinc-900'}`
                      : isDark
                        ? 'text-zinc-300'
                        : 'text-zinc-700'
                  }`}
                >
                  {cell}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
