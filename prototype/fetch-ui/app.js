
const state = {
  currentView: 'new',
  setupShown: false,
};

const pageMeta = {
  new: ['New download', 'Paste any URL supported by yt-dlp.'],
  downloads: ['Downloads', 'Monitor active jobs and queued items.'],
  completed: ['Completed', 'Browse and download finished files.'],
  history: ['History', 'Completed, failed and cancelled jobs.'],
  settings: ['Settings', 'Application, network and runtime preferences.'],
  logs: ['Logs', 'Application, yt-dlp and FFmpeg output.'],
};

function refreshIcons() {
  if (window.lucide) lucide.createIcons();
}

function showToast(text) {
  const toast = document.getElementById('toast');
  document.getElementById('toastText').textContent = text;
  toast.classList.add('show');
  setTimeout(() => toast.classList.remove('show'), 2200);
}

function navigate(view) {
  state.currentView = view;

  document.querySelectorAll('[data-view-panel]').forEach(el => {
    el.classList.toggle('is-active', el.dataset.viewPanel === view);
  });

  document.querySelectorAll('.nav-item, .mobile-nav').forEach(el => {
    el.classList.toggle('is-active', el.dataset.view === view);
  });

  const [title, subtitle] = pageMeta[view] || ['', ''];
  document.getElementById('pageTitle').textContent = title;
  document.getElementById('pageSubtitle').textContent = subtitle;
  document.querySelector('main').scrollTo({top: 0, behavior: 'smooth'});
}

document.addEventListener('click', e => {
  const nav = e.target.closest('[data-view]');
  if (nav) navigate(nav.dataset.view);

  const segment = e.target.closest('[data-media-type]');
  if (segment) {
    document.querySelectorAll('[data-media-type]').forEach(x => x.classList.remove('is-active'));
    segment.classList.add('is-active');
    showToast(segment.dataset.mediaType === 'audio' ? 'Audio mode selected' : 'Video mode selected');
  }

  const tab = e.target.closest('[data-settings-tab]');
  if (tab) {
    const target = tab.dataset.settingsTab;
    document.querySelectorAll('.settings-tab').forEach(x => x.classList.toggle('is-active', x === tab));
    document.querySelectorAll('.settings-panel').forEach(x => x.classList.toggle('is-active', x.dataset.settingsPanel === target));
  }

  const remove = e.target.closest('.remove-cidr');
  if (remove) remove.closest('.cidr-row').remove();

  const runtimeBtn = e.target.closest('.runtime-check');
  if (runtimeBtn) {
    const name = runtimeBtn.dataset.component;
    runtimeBtn.disabled = true;
    runtimeBtn.innerHTML = '<span class="inline-block size-3 animate-spin rounded-full border-2 border-zinc-600 border-t-teal-300"></span> Checking';
    setTimeout(() => {
      runtimeBtn.innerHTML = '<i data-lucide="check"></i> Up to date';
      runtimeBtn.disabled = false;
      refreshIcons();
      showToast(`${name} is up to date`);
    }, 1000);
  }
});

document.getElementById('analyzeBtn').addEventListener('click', () => {
  const input = document.getElementById('urlInput');
  if (!input.value.trim()) {
    input.focus();
    input.classList.add('ring-2', 'ring-rose-400/30');
    setTimeout(() => input.classList.remove('ring-2', 'ring-rose-400/30'), 700);
    showToast('Paste a media URL first');
    return;
  }

  const skeleton = document.getElementById('analysisSkeleton');
  const result = document.getElementById('analysisResult');
  result.classList.add('hidden');
  skeleton.classList.remove('hidden');

  setTimeout(() => {
    skeleton.classList.add('hidden');
    result.classList.remove('hidden');
    refreshIcons();
  }, 850);
});

document.getElementById('addDownloadBtn').addEventListener('click', () => {
  showToast('Added to download queue');
  setTimeout(() => navigate('downloads'), 450);
});

document.getElementById('addNetworkBtn').addEventListener('click', () => {
  const network = prompt('CIDR network', '10.0.0.0/24');
  if (!network) return;
  const row = document.createElement('div');
  row.className = 'cidr-row';
  row.innerHTML = `<code>${network}</code><button class="icon-btn size-8 remove-cidr"><i data-lucide="x"></i></button>`;
  document.getElementById('networkList').appendChild(row);
  refreshIcons();
});

document.getElementById('clearLogsBtn').addEventListener('click', () => {
  document.getElementById('logOutput').textContent = '';
  showToast('Logs cleared');
});

document.getElementById('serverButton').addEventListener('click', () => {
  navigate('settings');
  document.querySelector('[data-settings-tab="network"]').click();
});

document.getElementById('finishSetupBtn').addEventListener('click', () => {
  document.getElementById('bootstrapModal').classList.add('hidden');
  localStorage.setItem('fetch-prototype-setup', '1');
});

window.addEventListener('DOMContentLoaded', () => {
  refreshIcons();

  // Useful when demoing the prototype: add ?setup=1 to URL to force bootstrap modal.
  const params = new URLSearchParams(location.search);
  const forceSetup = params.get('setup') === '1';
  if (forceSetup || !localStorage.getItem('fetch-prototype-setup')) {
    document.getElementById('bootstrapModal').classList.remove('hidden');

    let pct = 68;
    const timer = setInterval(() => {
      pct = Math.min(100, pct + 4);
      document.getElementById('ffmpegSetupPct').textContent = pct === 100 ? 'Installed' : `${pct}%`;
      document.getElementById('ffmpegSetupBar').style.width = `${pct}%`;
      if (pct === 100) clearInterval(timer);
    }, 220);
  }

  document.getElementById('urlInput').value = 'https://www.youtube.com/watch?v=dQw4w9WgXcQ';
});
