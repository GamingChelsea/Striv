import { ReactNode, useState } from 'react'
import { ArrowLeft, ArrowRight, Menu, Moon, Search, Sun, Terminal, X } from 'lucide-react'
import {
  CHAPTER_GROUPS,
  CHAPTERS,
  ChapterId,
  DialectMode,
  LangMode,
  ThemeMode,
  tr,
} from '../types/docs'

interface LayoutProps {
  activeChapter: ChapterId
  dialect: DialectMode
  theme: ThemeMode
  lang: LangMode
  onNavigate: (chapter: ChapterId, sectionId?: string) => void
  onToggleDialect: (dialect: DialectMode) => void
  onToggleTheme: () => void
  onToggleLang: (lang: LangMode) => void
  onOpenSearch: () => void
  children: ReactNode
}

export function Layout({
  activeChapter,
  dialect,
  theme,
  lang,
  onNavigate,
  onToggleDialect,
  onToggleTheme,
  onToggleLang,
  onOpenSearch,
  children,
}: LayoutProps) {
  const [mobileOpen, setMobileOpen] = useState(false)
  const isDark = theme === 'dark'

  const currentIndex = CHAPTERS.findIndex((c) => c.id === activeChapter)
  const currentMeta = CHAPTERS[currentIndex] ?? CHAPTERS[0]
  const prevChapter = currentIndex > 0 ? CHAPTERS[currentIndex - 1] : null
  const nextChapter = currentIndex < CHAPTERS.length - 1 ? CHAPTERS[currentIndex + 1] : null

  const handleNav = (id: ChapterId, sectionId?: string) => {
    onNavigate(id, sectionId)
    setMobileOpen(false)
  }

  return (
    <div
      className={`min-h-screen flex flex-col transition-colors ${
        isDark ? 'bg-[#0d0f12] text-zinc-100' : 'bg-[#fafafa] text-zinc-900'
      }`}
    >
      {/* Top Bar */}
      <header
        className={`sticky top-0 z-30 h-14 border-b backdrop-blur-xs ${
          isDark
            ? 'border-zinc-800 bg-[#0d0f12]/95'
            : 'border-zinc-200 bg-[#fafafa]/95'
        }`}
      >
        <div className="max-w-[1440px] mx-auto h-full px-4 sm:px-6 flex items-center justify-between gap-3">
          {/* Left: Brand */}
          <div className="flex items-center gap-3">
            <button
              type="button"
              onClick={() => setMobileOpen((v) => !v)}
              aria-label={tr(lang, 'Toggle navigation', 'Kapitel-Navigation')}
              className={`lg:hidden p-1.5 rounded border cursor-pointer ${
                isDark
                  ? 'border-zinc-800 bg-zinc-900 text-zinc-300'
                  : 'border-zinc-200 bg-white text-zinc-700'
              }`}
            >
              {mobileOpen ? <X className="w-4 h-4" /> : <Menu className="w-4 h-4" />}
            </button>

            <button
              type="button"
              onClick={() => handleNav('overview')}
              className="flex items-center gap-2.5 cursor-pointer"
            >
              <span className="w-6 h-6 rounded border border-zinc-700 bg-zinc-900 text-amber-500 flex items-center justify-center">
                <Terminal className="w-3.5 h-3.5" />
              </span>
              <span className="font-mono font-semibold text-sm tracking-tight">Striv</span>
              <span
                className={`text-[11px] font-mono px-1.5 py-0.5 rounded border ${
                  isDark
                    ? 'border-zinc-800 bg-zinc-900 text-zinc-400'
                    : 'border-zinc-200 bg-white text-zinc-600'
                }`}
              >
                v1.2
              </span>
            </button>
          </div>

          {/* Right Controls: Reference, Search, EN/DE, Light/Dark, Rust/Odin */}
          <div className="flex items-center gap-2">
            {/* 0. Reference Docs (Rustdoc) */}
            <button
              type="button"
              onClick={() => onNavigate('reference')}
              className={`flex items-center gap-1.5 px-2.5 py-1.5 rounded border text-xs font-medium transition-colors cursor-pointer ${
                activeChapter === 'reference'
                  ? isDark
                    ? 'border-amber-500/60 bg-amber-500/15 text-amber-300'
                    : 'border-amber-400 bg-amber-50 text-amber-800'
                  : isDark
                    ? 'border-zinc-800 bg-zinc-900/90 hover:border-zinc-700 text-zinc-200'
                    : 'border-zinc-200 bg-white hover:border-zinc-300 text-zinc-700'
              }`}
            >
              <span>{tr(lang, 'Reference Docs', 'Referenz-Docs')}</span>
            </button>

            {/* 1. Search */}
            <button
              type="button"
              onClick={onOpenSearch}
              className={`flex items-center gap-2 px-2.5 py-1.5 rounded border text-xs transition-colors cursor-pointer ${
                isDark
                  ? 'border-zinc-800 bg-zinc-900/90 hover:border-zinc-700 text-zinc-400'
                  : 'border-zinc-200 bg-white hover:border-zinc-300 text-zinc-500'
              }`}
            >
              <Search className="w-3.5 h-3.5" />
              <span className="hidden sm:inline">{tr(lang, 'Search', 'Suche')}</span>
              <kbd
                className={`hidden md:inline-block font-mono text-[10px] px-1.5 py-0.5 rounded border ${
                  isDark
                    ? 'border-zinc-700 bg-zinc-800 text-zinc-400'
                    : 'border-zinc-200 bg-zinc-50 text-zinc-500'
                }`}
              >
                {tr(lang, 'Ctrl+K', 'Strg+K')}
              </kbd>
            </button>

            {/* 2. Language Switcher (EN | DE) */}
            <div
              className={`inline-flex rounded border p-0.5 text-xs font-mono ${
                isDark ? 'border-zinc-800 bg-zinc-900' : 'border-zinc-200 bg-white'
              }`}
              role="group"
              aria-label="Language"
            >
              <button
                type="button"
                onClick={() => onToggleLang('en')}
                className={`px-2 py-1 rounded-xs transition-colors cursor-pointer ${
                  lang === 'en'
                    ? isDark
                      ? 'bg-zinc-100 text-zinc-900 font-medium'
                      : 'bg-zinc-900 text-white font-medium'
                    : isDark
                      ? 'text-zinc-400 hover:text-zinc-200'
                      : 'text-zinc-600 hover:text-zinc-900'
                }`}
              >
                EN
              </button>
              <button
                type="button"
                onClick={() => onToggleLang('de')}
                className={`px-2 py-1 rounded-xs transition-colors cursor-pointer ${
                  lang === 'de'
                    ? isDark
                      ? 'bg-zinc-100 text-zinc-900 font-medium'
                      : 'bg-zinc-900 text-white font-medium'
                    : isDark
                      ? 'text-zinc-400 hover:text-zinc-200'
                      : 'text-zinc-600 hover:text-zinc-900'
                }`}
              >
                DE
              </button>
            </div>

            {/* 3. Light / Dark Mode Toggle */}
            <button
              type="button"
              onClick={onToggleTheme}
              aria-label={isDark ? 'Light Mode' : 'Dark Mode'}
              title={isDark ? 'Light Mode' : 'Dark Mode'}
              className={`p-1.5 rounded border transition-colors cursor-pointer ${
                isDark
                  ? 'border-zinc-800 bg-zinc-900 hover:border-zinc-700 text-zinc-300'
                  : 'border-zinc-200 bg-white hover:border-zinc-300 text-zinc-700'
              }`}
            >
              {isDark ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4" />}
            </button>

            {/* 4. Global Syntax Switcher (Rust | Odin) */}
            <div
              className={`inline-flex rounded border p-0.5 text-xs font-mono ${
                isDark ? 'border-zinc-800 bg-zinc-900' : 'border-zinc-200 bg-white'
              }`}
              role="group"
              aria-label="Syntax Dialect"
            >
              <button
                type="button"
                onClick={() => onToggleDialect('rust')}
                className={`px-2.5 py-1 rounded-xs transition-colors cursor-pointer ${
                  dialect === 'rust'
                    ? isDark
                      ? 'bg-zinc-100 text-zinc-900 font-medium'
                      : 'bg-zinc-900 text-white font-medium'
                    : isDark
                      ? 'text-zinc-400 hover:text-zinc-200'
                      : 'text-zinc-600 hover:text-zinc-900'
                }`}
              >
                {tr(lang, 'Rust', 'Rust-Stil')}
              </button>
              <button
                type="button"
                onClick={() => onToggleDialect('odin')}
                className={`px-2.5 py-1 rounded-xs transition-colors cursor-pointer ${
                  dialect === 'odin'
                    ? 'bg-amber-600 text-white font-medium'
                    : isDark
                      ? 'text-zinc-400 hover:text-zinc-200'
                      : 'text-zinc-600 hover:text-zinc-900'
                }`}
              >
                {tr(lang, 'Odin', 'Odin-Stil')}
              </button>
            </div>
          </div>
        </div>
      </header>

      {/* Mobile Drawer */}
      {mobileOpen && (
        <div
          className={`lg:hidden border-b px-4 py-4 space-y-4 ${
            isDark ? 'border-zinc-800 bg-[#12151a]' : 'border-zinc-200 bg-white'
          }`}
        >
          {CHAPTER_GROUPS.map((group) => (
            <div key={group.en} className="space-y-1">
              <div className="text-[11px] font-mono uppercase tracking-wider text-zinc-500">
                {group[lang]}
              </div>
              {CHAPTERS.filter((c) => c.group.en === group.en).map((c) => (
                <button
                  key={c.id}
                  type="button"
                  onClick={() => handleNav(c.id)}
                  className={`block w-full text-left text-sm px-2 py-1 rounded ${
                    c.id === activeChapter
                      ? isDark
                        ? 'bg-amber-500/15 text-amber-400 font-medium'
                        : 'bg-amber-50 text-amber-800 font-medium'
                      : isDark
                        ? 'text-zinc-300'
                        : 'text-zinc-700'
                  }`}
                >
                  {c.title[lang]}
                </button>
              ))}
            </div>
          ))}
        </div>
      )}

      {/* 3-Column Body */}
      <div className="max-w-[1440px] w-full mx-auto flex-1 flex">
        {/* Left: Fixed Scrollable Chapter Sidebar */}
        <aside
          className={`hidden lg:block w-68 shrink-0 border-r ${
            isDark ? 'border-zinc-800' : 'border-zinc-200'
          }`}
        >
          <div className="sticky top-14 max-h-[calc(100vh-3.5rem)] overflow-y-auto py-7 px-5 space-y-6">
            {CHAPTER_GROUPS.map((group) => (
              <div key={group.en} className="space-y-1.5">
                <div className="text-[11px] font-mono uppercase tracking-wider px-2 text-zinc-500">
                  {group[lang]}
                </div>
                <ul className="space-y-0.5">
                  {CHAPTERS.filter((c) => c.group.en === group.en).map((c) => {
                    const active = c.id === activeChapter
                    return (
                      <li key={c.id}>
                        <button
                          type="button"
                          onClick={() => handleNav(c.id)}
                          className={`w-full text-left text-[13px] px-2.5 py-1.5 rounded transition-colors cursor-pointer ${
                            active
                              ? isDark
                                ? 'bg-amber-500/15 text-amber-400 font-medium'
                                : 'bg-amber-50 text-amber-800 font-medium'
                              : isDark
                                ? 'text-zinc-400 hover:text-zinc-100 hover:bg-zinc-900'
                                : 'text-zinc-600 hover:text-zinc-900 hover:bg-zinc-100'
                          }`}
                        >
                          {c.title[lang]}
                        </button>
                      </li>
                    )
                  })}
                </ul>
              </div>
            ))}
          </div>
        </aside>

        {/* Center: Main Documentation Prose */}
        <main className="flex-1 min-w-0 px-5 sm:px-10 lg:px-12 py-8 sm:py-10">
          <div className="max-w-3xl mx-auto">
            <div
              className={`mb-6 pb-3 border-b flex flex-wrap items-center justify-between gap-2 text-xs font-mono ${
                isDark ? 'border-zinc-800 text-zinc-500' : 'border-zinc-200 text-zinc-500'
              }`}
            >
              <span>
                {currentMeta.group[lang]} /{' '}
                <span className={isDark ? 'text-zinc-200' : 'text-zinc-900'}>
                  {currentMeta.title[lang]}
                </span>
              </span>
              <span>
                {tr(lang, 'Dialect', 'Dialekt')}:{' '}
                {dialect === 'odin' ? 'syntax = "odin"' : 'syntax = "rust"'}
              </span>
            </div>

            <article className="space-y-10">{children}</article>

            {/* Prev / Next Chapter Footer */}
            <div
              className={`mt-14 pt-6 border-t grid grid-cols-1 sm:grid-cols-2 gap-4 ${
                isDark ? 'border-zinc-800' : 'border-zinc-200'
              }`}
            >
              {prevChapter ? (
                <button
                  type="button"
                  onClick={() => handleNav(prevChapter.id)}
                  className={`p-3.5 rounded-md border text-left transition-colors cursor-pointer ${
                    isDark
                      ? 'border-zinc-800 hover:border-zinc-700 bg-zinc-900/40'
                      : 'border-zinc-200 hover:border-zinc-300 bg-white'
                  }`}
                >
                  <div className="text-[11px] font-mono text-zinc-500 flex items-center gap-1">
                    <ArrowLeft className="w-3 h-3" />{' '}
                    {tr(lang, 'Previous Chapter', 'Vorheriges Kapitel')}
                  </div>
                  <div className="mt-1 text-sm font-medium text-amber-600">
                    {prevChapter.title[lang]}
                  </div>
                </button>
              ) : (
                <div />
              )}

              {nextChapter ? (
                <button
                  type="button"
                  onClick={() => handleNav(nextChapter.id)}
                  className={`p-3.5 rounded-md border text-right transition-colors cursor-pointer ${
                    isDark
                      ? 'border-zinc-800 hover:border-zinc-700 bg-zinc-900/40'
                      : 'border-zinc-200 hover:border-zinc-300 bg-white'
                  }`}
                >
                  <div className="text-[11px] font-mono text-zinc-500 flex items-center justify-end gap-1">
                    {tr(lang, 'Next Chapter', 'Nächstes Kapitel')}{' '}
                    <ArrowRight className="w-3 h-3" />
                  </div>
                  <div className="mt-1 text-sm font-medium text-amber-600">
                    {nextChapter.title[lang]}
                  </div>
                </button>
              ) : (
                <div />
              )}
            </div>
          </div>
        </main>

        {/* Right: "On this page" / "Auf dieser Seite" Table of Contents */}
        <aside className="hidden xl:block w-60 shrink-0 py-10 pr-6">
          <div className="sticky top-24 space-y-2.5">
            <div className="text-[11px] font-mono uppercase tracking-wider text-zinc-500">
              {tr(lang, 'On this page', 'Auf dieser Seite')}
            </div>
            <ul
              className={`space-y-1.5 border-l pl-3 text-xs ${
                isDark ? 'border-zinc-800' : 'border-zinc-200'
              }`}
            >
              {currentMeta.toc.map((item) => (
                <li key={item.id}>
                  <button
                    type="button"
                    onClick={() => handleNav(activeChapter, item.id)}
                    className={`text-left transition-colors cursor-pointer ${
                      isDark
                        ? 'text-zinc-400 hover:text-amber-400'
                        : 'text-zinc-600 hover:text-amber-700'
                    }`}
                  >
                    {item.label[lang]}
                  </button>
                </li>
              ))}
            </ul>
          </div>
        </aside>
      </div>
    </div>
  )
}
