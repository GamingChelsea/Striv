import { useEffect, useMemo, useRef, useState } from 'react'
import { ArrowRight, CornerDownLeft, Search, X } from 'lucide-react'
import { SEARCH_INDEX, SearchEntry } from '../data/searchIndex'
import { ChapterId, LangMode, ThemeMode, tr } from '../types/docs'

interface SearchModalProps {
  open: boolean
  theme: ThemeMode
  lang: LangMode
  onClose: () => void
  onNavigate: (chapter: ChapterId, sectionId?: string) => void
}

export function SearchModal({ open, theme, lang, onClose, onNavigate }: SearchModalProps) {
  const [query, setQuery] = useState('')
  const [activeIdx, setActiveIdx] = useState(0)
  const inputRef = useRef<HTMLInputElement>(null)
  const isDark = theme === 'dark'

  useEffect(() => {
    if (open) {
      setQuery('')
      setActiveIdx(0)
      requestAnimationFrame(() => inputRef.current?.focus())
    }
  }, [open])

  const results = useMemo(() => {
    const q = query.trim().toLowerCase()
    if (!q) return SEARCH_INDEX.slice(0, 10)
    return SEARCH_INDEX.filter((item) => {
      const hay =
        `${item.title[lang]} ${item.chapterLabel[lang]} ${item.summary[lang]} ${item.keywords.join(' ')}`.toLowerCase()
      return q.split(/\s+/).every((part) => hay.includes(part))
    })
  }, [query, lang])

  useEffect(() => {
    setActiveIdx(0)
  }, [results.length])

  const handleSelect = (entry: SearchEntry) => {
    onNavigate(entry.chapter, entry.sectionId)
    onClose()
  }

  useEffect(() => {
    if (!open) return
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault()
        onClose()
      } else if (e.key === 'ArrowDown') {
        e.preventDefault()
        setActiveIdx((prev) => (results.length ? (prev + 1) % results.length : 0))
      } else if (e.key === 'ArrowUp') {
        e.preventDefault()
        setActiveIdx((prev) =>
          results.length ? (prev - 1 + results.length) % results.length : 0,
        )
      } else if (e.key === 'Enter' && results[activeIdx]) {
        e.preventDefault()
        handleSelect(results[activeIdx])
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [open, results, activeIdx])

  if (!open) return null

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center pt-[12vh] px-4 bg-zinc-950/55"
      onClick={onClose}
      role="dialog"
      aria-modal="true"
      aria-label={tr(lang, 'Search documentation', 'Dokumentation durchsuchen')}
    >
      <div
        className={`w-full max-w-2xl rounded-md border shadow-xl overflow-hidden ${
          isDark
            ? 'border-zinc-700 bg-[#12151a] text-zinc-100'
            : 'border-zinc-300 bg-[#fafafa] text-zinc-900'
        }`}
        onClick={(e) => e.stopPropagation()}
      >
        <div
          className={`flex items-center gap-3 px-4 py-3 border-b ${
            isDark ? 'border-zinc-800 bg-zinc-900' : 'border-zinc-200 bg-white'
          }`}
        >
          <Search className="w-4 h-4 text-zinc-400 shrink-0" />
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={tr(
              lang,
              'Search all 14 chapters (Ecs, unit, sync, rpc, keywords, striv new)...',
              'Suche in allen 14 Kapiteln (Ecs, unit, sync, rpc, Keywords, striv new)...',
            )}
            className="w-full bg-transparent text-sm placeholder:text-zinc-400 focus:outline-none"
          />
          <button
            type="button"
            onClick={onClose}
            aria-label={tr(lang, 'Close search', 'Suche schließen')}
            className="p-1 text-zinc-400 hover:text-zinc-200 rounded cursor-pointer"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div
          className={`max-h-[56vh] overflow-y-auto divide-y ${
            isDark ? 'divide-zinc-800/80' : 'divide-zinc-200/80'
          }`}
        >
          {results.length === 0 ? (
            <div className="px-4 py-10 text-center text-sm text-zinc-500">
              {tr(lang, 'No results for', 'Keine Treffer für')}{' '}
              <span className="font-mono">„{query}“</span>.
            </div>
          ) : (
            results.map((entry, idx) => {
              const active = idx === activeIdx
              return (
                <button
                  key={entry.id}
                  type="button"
                  onMouseEnter={() => setActiveIdx(idx)}
                  onClick={() => handleSelect(entry)}
                  className={`w-full text-left px-4 py-3 flex items-start justify-between gap-4 transition-colors cursor-pointer ${
                    active
                      ? isDark
                        ? 'bg-amber-500/15'
                        : 'bg-amber-50'
                      : isDark
                        ? 'hover:bg-zinc-900'
                        : 'hover:bg-zinc-100'
                  }`}
                >
                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <span
                        className={`text-[11px] font-mono px-1.5 py-0.5 rounded border ${
                          isDark
                            ? 'border-zinc-700 bg-zinc-800 text-zinc-300'
                            : 'border-zinc-200 bg-white text-zinc-600'
                        }`}
                      >
                        {entry.chapterLabel[lang]}
                      </span>
                      <span className="text-sm font-medium truncate">{entry.title[lang]}</span>
                    </div>
                    <p className="mt-1 text-xs text-zinc-500 line-clamp-1">
                      {entry.summary[lang]}
                    </p>
                  </div>
                  <ArrowRight
                    className={`w-4 h-4 mt-1 shrink-0 ${
                      active ? 'text-amber-600' : 'text-zinc-400'
                    }`}
                  />
                </button>
              )
            })
          )}
        </div>

        <div
          className={`flex items-center justify-between px-4 py-2 border-t text-[11px] font-mono text-zinc-500 ${
            isDark ? 'border-zinc-800 bg-zinc-900/70' : 'border-zinc-200 bg-zinc-100/70'
          }`}
        >
          <div className="flex items-center gap-4">
            <span>↑↓ {tr(lang, 'navigate', 'navigieren')}</span>
            <span className="inline-flex items-center gap-1">
              <CornerDownLeft className="w-3 h-3" /> {tr(lang, 'open', 'öffnen')}
            </span>
          </div>
          <span>ESC {tr(lang, 'close', 'schließen')}</span>
        </div>
      </div>
    </div>
  )
}
