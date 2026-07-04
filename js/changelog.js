let currentLang = 'es';
let showOlderReleases = false;
let changelogData = [];

// i18n
function updateLanguage(lang) {
  if (typeof translations === 'undefined' || !translations[lang]) return;
  currentLang = lang;
  document.documentElement.setAttribute('lang', lang);

  // Elements text
  document.querySelectorAll('[data-i18n]').forEach(el => {
    const key = el.getAttribute('data-i18n');
    const value = translations[lang][key];
    if (value !== undefined) {
      if (value.includes('<') || value.includes('◆')) {
        el.innerHTML = value;
      } else {
        el.textContent = value;
      }
    }
  });

  // Toggle button text
  const btnToggleText = document.getElementById('btn-toggle-text');
  if (btnToggleText) {
    const key = showOlderReleases ? 'btn_hide_older' : 'btn_show_older';
    btnToggleText.setAttribute('data-i18n', key);
    btnToggleText.textContent = translations[lang][key];
  }

  // Title & Metatags
  const pageTitle = translations[lang].changelog_seo_title || (translations[lang].title + " - Changelog");
  const pageDesc = translations[lang].changelog_seo_desc || translations[lang].description;
  
  document.title = pageTitle;
  const metaDesc = document.querySelector('meta[name="description"]');
  if (metaDesc) {
    metaDesc.setAttribute('content', pageDesc);
  }

  const ogTitle = document.querySelector('meta[property="og:title"]');
  if (ogTitle) ogTitle.setAttribute('content', pageTitle);

  const ogDesc = document.querySelector('meta[property="og:description"]');
  if (ogDesc) ogDesc.setAttribute('content', pageDesc);

  const twTitle = document.querySelector('meta[name="twitter:title"]');
  if (twTitle) twTitle.setAttribute('content', pageTitle);

  const twDesc = document.querySelector('meta[name="twitter:description"]');
  if (twDesc) twDesc.setAttribute('content', pageDesc);

  const shareImgUrl = lang === 'es' ? 'https://pairee.fitty.ar/assets/Pairee_ES_2160x2160.png' : 'https://pairee.fitty.ar/assets/Pairee_EN_2160x2160.png';

  const ogImage = document.querySelector('meta[property="og:image"]');
  if (ogImage) ogImage.setAttribute('content', shareImgUrl);

  const twitterImage = document.querySelector('meta[name="twitter:image"]');
  if (twitterImage) twitterImage.setAttribute('content', shareImgUrl);

  // Active state visual language button
  document.querySelectorAll('.lang-btn').forEach(btn => {
    if (btn.getAttribute('data-lang') === lang) {
      btn.classList.add('active');
    } else {
      btn.classList.remove('active');
    }
  });

  // Re-render changelog if it's already loaded to apply new translation strings
  if (changelogData.length > 0) {
    renderChangelog(changelogData);
  }
}

function switchLanguage(lang) {
  localStorage.setItem('preferred-lang', lang);
  updateLanguage(lang);
}

// Changelog Loading & Parsing
async function fetchAndRenderChangelog() {
  const statusEl = document.getElementById('changelog-status');
  const actionsEl = document.getElementById('changelog-actions');
  
  try {
    const response = await fetch('https://raw.githubusercontent.com/FittyAr/Pairee/master/docs/CHANGELOG.md');
    if (!response.ok) throw new Error('Failed to fetch CHANGELOG.md');
    const mdText = await response.text();
    
    changelogData = parseChangelogMarkdown(mdText);
    if (changelogData.length === 0) {
      throw new Error('No releases parsed');
    }

    renderChangelog(changelogData);

    if (changelogData.length > 3) {
      actionsEl.style.display = 'flex';
    }
  } catch (error) {
    console.error('Error loading changelog:', error);
    if (statusEl) {
      statusEl.setAttribute('data-i18n', 'changelog_error');
      statusEl.textContent = translations[currentLang]?.changelog_error || 'No se pudo cargar el historial de cambios.';
    }
  }
}

function renderChangelog(releases) {
  const container = document.getElementById('changelog-container');
  if (!container) return;
  container.innerHTML = '';

  releases.forEach((release, idx) => {
    const isOlder = idx >= 3;
    const itemEl = document.createElement('div');
    itemEl.className = 'changelog-item' + (isOlder ? ' older-release' : '');
    
    let categoriesHtml = '';
    for (const [catName, items] of Object.entries(release.categories)) {
      if (items.length === 0) continue;
      
      let catClass = catName.toLowerCase();
      let catTitle = catName;
      
      // Localize category labels dynamically
      const translationKey = 'changelog_cat_' + catClass;
      if (translations[currentLang] && translations[currentLang][translationKey]) {
        catTitle = translations[currentLang][translationKey];
      }

      categoriesHtml += `
        <div class="changelog-category">
          <span class="changelog-category-title ${catClass}">${catTitle}</span>
          <ul class="changelog-list">
            ${items.map(item => `<li>${escapeHtml(item)}</li>`).join('')}
          </ul>
        </div>
      `;
    }

    const latestLabel = translations[currentLang]?.changelog_latest || 'LATEST';
    const dateLabel = release.date === 'Unreleased' ? (translations[currentLang]?.changelog_unreleased || 'Unreleased') : release.date;

    itemEl.innerHTML = `
      <div class="changelog-dot">${idx + 1}</div>
      <div class="changelog-card">
        <div class="changelog-header">
          <div class="changelog-version">
            ${escapeHtml(release.version)}
            ${idx === 0 ? `<span style="background: rgba(0, 229, 255, 0.1); border-color: var(--accent-cyan); color: var(--accent-cyan); font-size: 0.7rem; margin-left: 10px; border-radius: 4px; padding: 2px 6px;">${latestLabel}</span>` : ''}
          </div>
          <div class="changelog-date">${escapeHtml(dateLabel)}</div>
        </div>
        <div class="changelog-body">
          ${categoriesHtml}
        </div>
      </div>
    `;
    container.appendChild(itemEl);
  });

  // Maintain toggle older state
  const olderItems = document.querySelectorAll('.changelog-item.older-release');
  olderItems.forEach(item => {
    if (showOlderReleases) {
      item.classList.add('show');
    } else {
      item.classList.remove('show');
    }
  });
}

function parseChangelogMarkdown(text) {
  const lines = text.split('\n');
  const releases = [];
  let currentRelease = null;
  let currentCategory = null;

  for (let line of lines) {
    line = line.trim();
    if (!line) continue;

    const versionMatch = line.match(/^##\s+\[([^\]]+)\](?:\s*-\s*(\d{4}-\d{2}-\d{2}))?/);
    if (versionMatch) {
      currentRelease = {
        version: versionMatch[1],
        date: versionMatch[2] || 'Unreleased',
        categories: {
          'Added': [],
          'Changed': [],
          'Improved': [],
          'Fixed': [],
          'Removed': []
        }
      };
      releases.push(currentRelease);
      currentCategory = null;
      continue;
    }

    const categoryMatch = line.match(/^###\s+(\w+)/);
    if (categoryMatch && currentRelease) {
      const cat = categoryMatch[1];
      if (cat === 'Added' || cat === 'Changed' || cat === 'Improved' || cat === 'Fixed' || cat === 'Removed') {
        currentCategory = cat;
      } else {
        currentRelease.categories[cat] = [];
        currentCategory = cat;
      }
      continue;
    }

    const itemMatch = line.match(/^-\s+(.+)$/);
    if (itemMatch && currentRelease && currentCategory) {
      currentRelease.categories[currentCategory].push(itemMatch[1]);
    }
  }

  return releases;
}

function toggleOlderReleases() {
  showOlderReleases = !showOlderReleases;
  const olderItems = document.querySelectorAll('.changelog-item.older-release');
  olderItems.forEach(item => {
    if (showOlderReleases) {
      item.classList.add('show');
    } else {
      item.classList.remove('show');
    }
  });

  const btnTextEl = document.getElementById('btn-toggle-text');
  const btnIconEl = document.getElementById('btn-toggle-icon');
  if (btnTextEl && translations[currentLang]) {
    const key = showOlderReleases ? 'btn_hide_older' : 'btn_show_older';
    btnTextEl.setAttribute('data-i18n', key);
    btnTextEl.textContent = translations[currentLang][key];
  }
  if (btnIconEl) {
    btnIconEl.textContent = showOlderReleases ? '▲' : '▼';
  }
}

function escapeHtml(text) {
  if (!text) return '';
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

// Init
document.addEventListener('DOMContentLoaded', () => {
  let lang = localStorage.getItem('preferred-lang');
  if (!lang) {
    const browserLang = navigator.language || navigator.userLanguage;
    lang = (browserLang && browserLang.startsWith('en')) ? 'en' : 'es';
  }
  updateLanguage(lang);
  fetchAndRenderChangelog();
});
