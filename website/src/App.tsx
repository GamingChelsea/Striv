import { useCallback, useEffect, useState } from 'react'
import {
  BasicsChapter,
  EnumsGenericsChapter,
  StructsChapter,
  UnitsChapter,
} from './chapters/FundamentalsAndTypes'
import {
  CliChapter,
  FromOdinChapter,
  FromRustChapter,
  OverviewChapter,
} from './chapters/IntroAndGuides'
import { KeywordsTypesChapter } from './chapters/KeywordsTypesChapter'
import {
  ConcurrencyChapter,
  EcsChapter,
  NetworkingChapter,
} from './chapters/ConcurrencyEcsNet'
import {
  ModdingInteropChapter,
  StdlibTestingChapter,
} from './chapters/ModdingAndStdlib'
import { Layout } from './components/Layout'
import { SearchModal } from './components/SearchModal'
import { CHAPTERS, ChapterId, DialectMode, LangMode, ThemeMode } from './types/docs'

function parseInitialChapter(): ChapterId {
  const hash = window.location.hash.replace('#', '').split('/')[0]
  if (CHAPTERS.some((c) => c.id === hash)) {
    return hash as ChapterId
  }
  return 'overview'
}

export function App() {
  const [activeChapter, setActiveChapter] = useState<ChapterId>(parseInitialChapter)
  const [dialect, setDialect] = useState<DialectMode>(() => {
    const saved = localStorage.getItem('striv_docs_dialect')
    return saved === 'odin' ? 'odin' : 'rust'
  })
  const [theme, setTheme] = useState<ThemeMode>(() => {
    const saved = localStorage.getItem('striv_docs_theme')
    return saved === 'dark' ? 'dark' : 'light'
  })
  const [lang, setLang] = useState<LangMode>(() => {
    const saved = localStorage.getItem('striv_docs_lang')
    return saved === 'de' ? 'de' : 'en'
  })
  const [searchOpen, setSearchOpen] = useState(false)

  const handleToggleDialect = useCallback((next: DialectMode) => {
    setDialect(next)
    localStorage.setItem('striv_docs_dialect', next)
  }, [])

  const handleToggleTheme = useCallback(() => {
    setTheme((prev) => {
      const next = prev === 'dark' ? 'light' : 'dark'
      localStorage.setItem('striv_docs_theme', next)
      return next
    })
  }, [])

  const handleToggleLang = useCallback((next: LangMode) => {
    setLang(next)
    localStorage.setItem('striv_docs_lang', next)
  }, [])

  const navigate = useCallback((chapter: ChapterId, sectionId?: string) => {
    setActiveChapter(chapter)
    window.history.replaceState(null, '', sectionId ? `#${chapter}/${sectionId}` : `#${chapter}`)
    requestAnimationFrame(() => {
      if (sectionId) {
        const el = document.getElementById(sectionId)
        if (el) {
          el.scrollIntoView({ behavior: 'smooth', block: 'start' })
          return
        }
      }
      window.scrollTo({ top: 0, behavior: 'auto' })
    })
  }, [])

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault()
        setSearchOpen((prev) => !prev)
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [])

  return (
    <>
      <Layout
        activeChapter={activeChapter}
        dialect={dialect}
        theme={theme}
        lang={lang}
        onNavigate={navigate}
        onToggleDialect={handleToggleDialect}
        onToggleTheme={handleToggleTheme}
        onToggleLang={handleToggleLang}
        onOpenSearch={() => setSearchOpen(true)}
      >
        {activeChapter === 'overview' && (
          <OverviewChapter dialect={dialect} theme={theme} lang={lang} />
        )}
        {activeChapter === 'cli' && <CliChapter dialect={dialect} theme={theme} lang={lang} />}
        {activeChapter === 'from-rust' && (
          <FromRustChapter dialect={dialect} theme={theme} lang={lang} />
        )}
        {activeChapter === 'from-odin' && (
          <FromOdinChapter dialect={dialect} theme={theme} lang={lang} />
        )}
        {activeChapter === 'basics' && (
          <BasicsChapter dialect={dialect} theme={theme} lang={lang} />
        )}
        {activeChapter === 'structs' && (
          <StructsChapter dialect={dialect} theme={theme} lang={lang} />
        )}
        {activeChapter === 'enums-generics' && (
          <EnumsGenericsChapter dialect={dialect} theme={theme} lang={lang} />
        )}
        {activeChapter === 'units' && (
          <UnitsChapter dialect={dialect} theme={theme} lang={lang} />
        )}
        {activeChapter === 'keywords-types' && (
          <KeywordsTypesChapter dialect={dialect} theme={theme} lang={lang} />
        )}
        {activeChapter === 'concurrency' && (
          <ConcurrencyChapter dialect={dialect} theme={theme} lang={lang} />
        )}
        {activeChapter === 'ecs' && <EcsChapter dialect={dialect} theme={theme} lang={lang} />}
        {activeChapter === 'networking' && (
          <NetworkingChapter dialect={dialect} theme={theme} lang={lang} />
        )}
        {activeChapter === 'modding-interop' && (
          <ModdingInteropChapter dialect={dialect} theme={theme} lang={lang} />
        )}
        {activeChapter === 'stdlib-testing' && (
          <StdlibTestingChapter dialect={dialect} theme={theme} lang={lang} />
        )}
      </Layout>

      <SearchModal
        open={searchOpen}
        theme={theme}
        lang={lang}
        onClose={() => setSearchOpen(false)}
        onNavigate={navigate}
      />
    </>
  )
}

export default App
