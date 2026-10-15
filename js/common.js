// Shared by every page: language switching and the release badge.
let currentLang = 'es';
let latestRelease = null;
const onLanguageChange = [];

function t(key) {
  return translations[currentLang]?.[key] ?? translations.en[key] ?? key;
}

function setMeta(selector, value) {
  const el = document.querySelector(selector);
  if (el && value) el.setAttribute('content', value);
}

function updateLanguage(lang) {
  if (typeof translations === 'undefined' || !translations[lang]) return;
  currentLang = lang;
  document.documentElement.setAttribute('lang', lang);

  document.querySelectorAll('[data-i18n]').forEach(el => {
    const value = translations[lang][el.getAttribute('data-i18n')];
    if (typeof value !== 'string') return;
    if (value.includes('<')) el.innerHTML = value;
    else el.textContent = value;
  });
  document.querySelectorAll('[data-i18n-alt]').forEach(el => {
    const value = translations[lang][el.getAttribute('data-i18n-alt')];
    if (value) el.setAttribute('alt', value);
  });

  const page = document.body.dataset.page;
  const title = page ? t(`${page}_seo_title`) : t('title');
  const desc = page ? t(`${page}_seo_desc`) : t('description');
  document.title = title;
  setMeta('meta[name="description"]', desc);
  setMeta('meta[property="og:title"]', title);
  setMeta('meta[property="og:description"]', desc);
  setMeta('meta[name="twitter:title"]', title);
  setMeta('meta[name="twitter:description"]', desc);

  document.querySelectorAll('.lang-btn').forEach(btn => {
    btn.classList.toggle('active', btn.getAttribute('data-lang') === lang);
  });

  updateReleaseBadge();
  onLanguageChange.forEach(fn => fn(lang));
}

function switchLanguage(lang) {
  try { localStorage.setItem('preferred-lang', lang); } catch (_) { /* storage blocked */ }
  updateLanguage(lang);
}

function preferredLanguage() {
  let lang = null;
  try { lang = localStorage.getItem('preferred-lang'); } catch (_) { /* storage blocked */ }
  if (!lang) {
    const browserLang = navigator.language || '';
    lang = browserLang.startsWith('es') ? 'es' : 'en';
  }
  return lang;
}

// The newest published release (pre-releases included, drafts excluded).
async function fetchLatestRelease() {
  if (!document.getElementById('release-badge')) return;
  try {
    const response = await fetch('https://api.github.com/repos/FittyAr/Pairee/releases?per_page=5');
    if (!response.ok) throw new Error('API request failed');
    const releases = (await response.json()).filter(r => !r.draft);
    if (releases.length === 0) throw new Error('No releases');
    const r = releases[0];
    const days = (Date.now() - new Date(r.published_at).getTime()) / 86400000;
    latestRelease = { tag: r.tag_name, url: r.html_url, isNew: days <= 30, pre: r.prerelease };
  } catch (error) {
    console.error('Error fetching release data:', error);
    latestRelease = {
      tag: 'v0.11.0', // Newest known release, used when the API is unavailable.
      url: 'https://github.com/FittyAr/Pairee/releases',
      isNew: false,
      pre: false
    };
  }
  updateReleaseBadge();
}

function updateReleaseBadge() {
  const textEl = document.getElementById('release-text');
  const linkEl = document.querySelector('.release-link');
  if (!latestRelease || !textEl) return;
  if (linkEl) linkEl.href = latestRelease.url;
  const key = latestRelease.pre ? 'release_pre' : latestRelease.isNew ? 'release_new' : 'release_old';
  textEl.textContent = t(key).replace('{version}', latestRelease.tag);
}

document.addEventListener('DOMContentLoaded', () => {
  updateLanguage(preferredLanguage());
  fetchLatestRelease();
});
