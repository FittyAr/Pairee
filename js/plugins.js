let currentLang = 'es';
let pluginsData = [];
let activeFilter = 'all';

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

  // Placeholders
  document.querySelectorAll('[data-i18n-placeholder]').forEach(el => {
    const key = el.getAttribute('data-i18n-placeholder');
    const value = translations[lang][key];
    if (value !== undefined) {
      el.setAttribute('placeholder', value);
    }
  });

  // Title & Metatags
  const pageTitle = translations[lang].plugins_seo_title || (translations[lang].title + " - Plugins");
  const pageDesc = translations[lang].plugins_seo_desc || translations[lang].description;
  
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
}

function switchLanguage(lang) {
  localStorage.setItem('preferred-lang', lang);
  updateLanguage(lang);
  filterPlugins();
}

// Plugins Registry Loading & Parsing
async function fetchAndRenderPlugins() {
  const grid = document.getElementById('store-grid');
  
  try {
    const response = await fetch('https://raw.githubusercontent.com/FittyAr/Pairee/plugin-registry/registry/index.toml');
    if (!response.ok) throw new Error('Failed to fetch registry index.toml');
    const tomlText = await response.text();
    
    const parsed = parseTOML(tomlText);
    pluginsData = Object.values(parsed.plugins || {});
    
    renderPlugins(pluginsData);
  } catch (error) {
    console.error('Error loading plugins:', error);
    grid.innerHTML = `
      <div class="store-empty">
        <span class="store-empty-icon">❌</span>
        <span data-i18n="store_error">${translations[currentLang]?.store_error || 'No se pudo cargar la lista de plugins.'}</span>
      </div>
    `;
  }
}

function parseTOML(text) {
  const lines = text.split('\n');
  const data = {};
  let currentSection = null;

  for (let line of lines) {
    line = line.trim();
    if (!line || line.startsWith('#')) continue;

    const sectionMatch = line.match(/^\[([^\]]+)\]$/);
    if (sectionMatch) {
      const sectionName = sectionMatch[1].trim();
      if (sectionName === "plugins") {
        if (!data.plugins) data.plugins = {};
        currentSection = data.plugins;
      } else if (sectionName.startsWith("plugins.")) {
        if (!data.plugins) data.plugins = {};
        let pluginName = sectionName.substring(8).trim();
        if (pluginName.startsWith('"') && pluginName.endsWith('"')) {
          pluginName = pluginName.slice(1, -1);
        }
        if (!data.plugins[pluginName]) {
          data.plugins[pluginName] = {};
        }
        currentSection = data.plugins[pluginName];
      } else {
        const parts = sectionName.split('.');
        let current = data;
        for (let i = 0; i < parts.length; i++) {
          let part = parts[i].trim();
          if (part.startsWith('"') && part.endsWith('"')) {
            part = part.slice(1, -1);
          }
          if (!current[part]) {
            current[part] = {};
          }
          current = current[part];
        }
        currentSection = current;
      }
      continue;
    }

    const kvMatch = line.match(/^([^=]+)=(.*)$/);
    if (kvMatch && currentSection) {
      const key = kvMatch[1].trim();
      let valueVal = kvMatch[2].trim();
      
      if (valueVal.includes('#')) {
        valueVal = valueVal.split('#')[0].trim();
      }

      let parsedValue = valueVal;
      if (valueVal.startsWith('"') && valueVal.endsWith('"')) {
        parsedValue = valueVal.slice(1, -1);
      } else if (valueVal.startsWith('[') && valueVal.endsWith(']')) {
        const arrContent = valueVal.slice(1, -1).trim();
        if (arrContent === "") {
          parsedValue = [];
        } else {
          parsedValue = arrContent.split(',').map(item => {
            item = item.trim();
            if (item.startsWith('"') && item.endsWith('"')) {
              return item.slice(1, -1);
            }
            return item;
          });
        }
      } else if (valueVal === "true") {
        parsedValue = true;
      } else if (valueVal === "false") {
        parsedValue = false;
      } else if (!isNaN(valueVal)) {
        parsedValue = Number(valueVal);
      }

      currentSection[key] = parsedValue;
    }
  }
  return data;
}

function renderPlugins(plugins) {
  const grid = document.getElementById('store-grid');
  grid.innerHTML = '';

  if (plugins.length === 0) {
    grid.innerHTML = `
      <div class="store-empty">
        <span class="store-empty-icon">🔍</span>
        <span data-i18n="store_no_results">${translations[currentLang]?.store_no_results || 'No se encontraron plugins.'}</span>
      </div>
    `;
    return;
  }

  const detailsBtnText = (typeof translations !== 'undefined' && translations[currentLang])
    ? (translations[currentLang].btn_plugin_details || 'Detalles')
    : 'Detalles';

  plugins.forEach(plugin => {
    const card = document.createElement('div');
    card.className = 'plugin-card';
    
    let isHook = false;
    let isCommand = false;
    let isPreviewer = false;
    if (plugin.hooks && plugin.hooks.length > 0) {
      plugin.hooks.forEach(h => {
        if (h === 'peek' || plugin.name.includes('previewer') || plugin.name.includes('peek')) {
          isPreviewer = true;
        } else if (h.startsWith('on_')) {
          isHook = true;
        } else {
          isCommand = true;
        }
      });
    } else {
      isCommand = true;
    }

    let typeClass = 'mixed';
    let typeLabel = 'Mixed';
    if (isPreviewer) {
      typeClass = 'previewer';
      typeLabel = 'Previewer';
    } else if (isHook && isCommand) {
      typeClass = 'mixed';
      typeLabel = 'Mixed';
    } else if (isHook) {
      typeClass = 'hook';
      typeLabel = 'Hook';
    } else {
      typeClass = 'command';
      typeLabel = 'Command';
    }

    // Translate type badges
    const badgeText = (translations[currentLang] && translations[currentLang]['badge_' + typeClass]) 
      ? translations[currentLang]['badge_' + typeClass] 
      : typeLabel;

    const langBadges = (plugin.languages || []).map(l => `<span class="plugin-meta-badge">${l.toUpperCase()}</span>`).join(' ');

    card.innerHTML = `
      <div class="plugin-card-header">
        <div class="plugin-title-area">
          <span class="plugin-name">${escapeHtml(cleanPluginName(plugin.name))}</span>
          <span class="plugin-author">by ${escapeHtml(plugin.author || 'unknown')}</span>
        </div>
        <span class="plugin-type-badge ${typeClass}">${escapeHtml(badgeText)}</span>
      </div>
      <p class="plugin-desc">${escapeHtml(plugin.description || 'No description provided.')}</p>
      <div class="plugin-card-footer">
        <div class="plugin-meta">
          <span class="plugin-meta-badge" style="background: rgba(0,229,255,0.08); border-color: rgba(0,229,255,0.2); color: var(--accent-cyan)">v${escapeHtml(plugin.version)}</span>
          ${langBadges}
        </div>
        <button class="btn-details" onclick="openPluginModal('${plugin.name}')" data-i18n="btn_plugin_details">${detailsBtnText}</button>
      </div>
    `;
    grid.appendChild(card);
  });
}

function filterPlugins() {
  const searchVal = document.getElementById('store-search').value.toLowerCase();
  
  const filtered = pluginsData.filter(plugin => {
    const nameMatch = cleanPluginName(plugin.name).toLowerCase().includes(searchVal);
    const authorMatch = (plugin.author || '').toLowerCase().includes(searchVal);
    const descMatch = (plugin.description || '').toLowerCase().includes(searchVal);
    
    const matchesSearch = nameMatch || authorMatch || descMatch;
    if (!matchesSearch) return false;
    if (activeFilter === 'all') return true;
    
    let isHook = false;
    let isCommand = false;
    let isPreviewer = false;
    if (plugin.hooks && plugin.hooks.length > 0) {
      plugin.hooks.forEach(h => {
        if (h === 'peek' || plugin.name.includes('previewer') || plugin.name.includes('peek')) {
          isPreviewer = true;
        } else if (h.startsWith('on_')) {
          isHook = true;
        } else {
          isCommand = true;
        }
      });
    } else {
      isCommand = true;
    }

    if (activeFilter === 'previewer') return isPreviewer;
    if (activeFilter === 'mixed') return !isPreviewer && isHook && isCommand;
    if (activeFilter === 'hook') return !isPreviewer && isHook && !isCommand;
    if (activeFilter === 'command') return !isPreviewer && !isHook && isCommand;
    
    return true;
  });

  renderPlugins(filtered);
}

function setPluginFilter(category) {
  activeFilter = category;
  
  document.querySelectorAll('#store-filters .filter-btn').forEach(btn => {
    if (btn.getAttribute('data-filter') === category) {
      btn.classList.add('active');
    } else {
      btn.classList.remove('active');
    }
  });
  
  filterPlugins();
}

async function openPluginModal(name) {
  const modal = document.getElementById('plugin-modal');
  const plugin = pluginsData.find(p => p.name === name);
  if (!plugin) return;

  const loadingLabel = translations[currentLang]?.modal_loading || 'Cargando detalles...';
  const noneLabel = translations[currentLang]?.modal_none || 'Ninguno';

  document.getElementById('modal-name').textContent = cleanPluginName(plugin.name);
  document.getElementById('modal-author-version').textContent = `by ${plugin.author || 'unknown'} • v${plugin.version}`;
  document.getElementById('modal-description').textContent = plugin.description || '';
  document.getElementById('modal-meta-author').textContent = plugin.author || 'unknown';
  document.getElementById('modal-meta-version').textContent = plugin.version;
  document.getElementById('modal-meta-min-version').textContent = plugin.min_pairee || 'N/A';
  
  document.getElementById('modal-meta-trust').innerHTML = `<span style="color: var(--text-muted)">${loadingLabel}</span>`;
  document.getElementById('modal-meta-languages').innerHTML = '';
  document.getElementById('modal-meta-hooks').innerHTML = '';
  document.getElementById('modal-keybindings-section').style.display = 'none';
  document.getElementById('modal-keybindings-tbody').innerHTML = '';

  modal.classList.add('active');

  try {
    const author = plugin.author || 'unknown';
    const firstChar = author.charAt(0).toLowerCase();
    const firstCharStr = /[a-z]/.test(firstChar) ? firstChar : '_';
    
    const manifestUrl = `https://raw.githubusercontent.com/FittyAr/Pairee/plugin-registry/registry/plugins/${firstCharStr}/${author}/${plugin.name}/manifest.toml`;
    
    const response = await fetch(manifestUrl);
    if (!response.ok) throw new Error('Manifest not found');
    const manifestToml = await response.text();
    const manifest = parseTOML(manifestToml);

    if (manifest.description) {
      document.getElementById('modal-description').textContent = manifest.description;
    }

    const requiresTrust = manifest.requires_trust === true;
    const trustEl = document.getElementById('modal-meta-trust');
    if (requiresTrust) {
      trustEl.className = 'modal-meta-value trust-badge yes';
      trustEl.innerHTML = `⚠️ <span data-i18n="modal_trust_yes">${translations[currentLang]?.modal_trust_yes || 'Yes'}</span>`;
    } else {
      trustEl.className = 'modal-meta-value trust-badge no';
      trustEl.innerHTML = `✓ <span data-i18n="modal_trust_no">${translations[currentLang]?.modal_trust_no || 'No'}</span>`;
    }

    const languages = manifest.languages || plugin.languages || [];
    const langContainer = document.getElementById('modal-meta-languages');
    langContainer.innerHTML = languages.map(l => `<span class="plugin-meta-badge">${l.toUpperCase()}</span>`).join(' ');

    const hooks = manifest.hooks || plugin.hooks || [];
    const hooksContainer = document.getElementById('modal-meta-hooks');
    if (hooks.length > 0) {
      hooksContainer.innerHTML = hooks.map(h => `<span class="plugin-meta-badge" style="background: rgba(46, 204, 113, 0.08); border-color: rgba(46, 204, 113, 0.2); color: #2ecc71;">${h}</span>`).join(' ');
    } else {
      hooksContainer.innerHTML = `<span style="font-size: 0.85rem; color: var(--text-muted);">${noneLabel}</span>`;
    }

    const keybindings = manifest.keybindings || {};
    const kbSection = document.getElementById('modal-keybindings-section');
    const kbTbody = document.getElementById('modal-keybindings-tbody');
    
    const keys = Object.keys(keybindings);
    if (keys.length > 0) {
      kbSection.style.display = 'block';
      kbTbody.innerHTML = '';
      keys.forEach(k => {
        const tr = document.createElement('tr');
        tr.innerHTML = `
          <td><code>${escapeHtml(k)}</code></td>
          <td style="font-family: 'JetBrains Mono', monospace; color: var(--text);">${escapeHtml(keybindings[k])}</td>
        `;
        kbTbody.appendChild(tr);
      });
    }

  } catch (error) {
    console.warn('Could not load detailed manifest.toml, showing basic registry info.', error);
    const trustEl = document.getElementById('modal-meta-trust');
    trustEl.className = 'modal-meta-value trust-badge no';
    trustEl.innerHTML = `✓ <span data-i18n="modal_trust_no">${translations[currentLang]?.modal_trust_no || 'No'}</span>`;
    
    const langContainer = document.getElementById('modal-meta-languages');
    langContainer.innerHTML = (plugin.languages || []).map(l => `<span class="plugin-meta-badge">${l.toUpperCase()}</span>`).join(' ');

    const hooksContainer = document.getElementById('modal-meta-hooks');
    if (plugin.hooks && plugin.hooks.length > 0) {
      hooksContainer.innerHTML = plugin.hooks.map(h => `<span class="plugin-meta-badge" style="background: rgba(46, 204, 113, 0.08); border-color: rgba(46, 204, 113, 0.2); color: #2ecc71;">${h}</span>`).join(' ');
    } else {
      hooksContainer.innerHTML = `<span style="font-size: 0.85rem; color: var(--text-muted);">${noneLabel}</span>`;
    }
  }
}

function closePluginModal() {
  document.getElementById('plugin-modal').classList.remove('active');
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

function cleanPluginName(name) {
  if (!name) return '';
  return name.endsWith('.pairee') ? name.slice(0, -7) : name;
}

// Init
document.addEventListener('DOMContentLoaded', () => {
  let lang = localStorage.getItem('preferred-lang');
  if (!lang) {
    const browserLang = navigator.language || navigator.userLanguage;
    lang = (browserLang && browserLang.startsWith('en')) ? 'en' : 'es';
  }
  updateLanguage(lang);
  fetchAndRenderPlugins();
});
