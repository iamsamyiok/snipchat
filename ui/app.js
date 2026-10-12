// app.js — 主窗：codex 聊天 + 图片附件
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const chat = document.getElementById('chat');
const prompt = document.getElementById('prompt');
const send = document.getElementById('send');
const attach = document.getElementById('attach');
const modeEl = document.getElementById('mode');

let images = [];        // [{path, dataUrl}]
let started = false;    // 本话题是否已发过首轮（决定 resume）
let running = false;
let transcript = [];    // [{role, text}] 供降级续聊

function bubble(cls, text) {
  const el = document.createElement('div');
  el.className = 'msg ' + cls;
  el.textContent = text;
  chat.appendChild(el);
  chat.scrollTop = chat.scrollHeight;
  return el;
}
function clearEmpty() { const e = chat.querySelector('.empty'); if (e) e.remove(); }

function renderAttach() {
  attach.innerHTML = '';
  attach.classList.toggle('show', images.length > 0);
  images.forEach((img, i) => {
    const d = document.createElement('div');
    d.className = 'att';
    d.innerHTML = `<img src="${img.dataUrl}"><button data-i="${i}">×</button>`;
    d.querySelector('button').onclick = () => { images.splice(i, 1); renderAttach(); };
    attach.appendChild(d);
  });
}

// 编辑器发来的「问问 codex」
listen('chat:attach', e => {
  const { path, dataUrl } = e.payload;
  images.push({ path, dataUrl });
  renderAttach();
  window.__TAURI__.window.getCurrentWindow().show();
  window.__TAURI__.window.getCurrentWindow().setFocus();
  prompt.focus();
});

async function doSend() {
  const text = prompt.value.trim();
  if (!text || running) return;
  clearEmpty();
  running = true; send.disabled = true;

  const el = bubble('user', text);
  if (images.length) {
    const box = document.createElement('div');
    box.className = 'imgs';
    for (const img of images) box.innerHTML += `<img src="${img.dataUrl}">`;
    el.prepend(box);
  }
  const bot = bubble('bot running', '… codex 处理中');
  window._lastUser = text;
  const userImgs = images.splice(0);
  renderAttach();
  prompt.value = '';

  try {
    await invoke('codex_send', {
      prompt: text,
      images: userImgs.map(i => i.path),
      resume: started,
    });
  } catch (err) {
    bot.textContent = String(err);
    bot.classList.remove('running');
    running = false; send.disabled = false;
    return;
  }
  // 流式输出由事件驱动（见下方 listen）
  window._curBot = bot;
  window._curBuf = [];
}

listen('codex:start', () => { window._curBuf = []; if (window._curBot) window._curBot.textContent = ''; });
listen('codex:delta', e => {
  window._curBuf.push(e.payload.text);
  if (window._curBot) {
    window._curBot.textContent = window._curBuf.join('\n');
    chat.scrollTop = chat.scrollHeight;
  }
});
listen('codex:note', e => bubble('sys', e.payload.text));
listen('codex:done', e => {
  if (window._curBot) {
    window._curBot.classList.remove('running');
    const full = (window._curBuf || []).join('\n') || '(无输出)';
    window._curBot.textContent = full;
    transcript.push({ role: 'user', text: window._lastUser || '' });
    transcript.push({ role: 'assistant', text: full.slice(0, 500) });
    invoke('codex_mirror_transcript', { rounds: transcript }).catch(() => {});
    started = true;
    modeEl.textContent = '续聊中 · 新话题可重开';
  }
  if (e.payload.error) bubble('sys', '⚠ ' + String(e.payload.error).slice(0, 160));
  running = false; send.disabled = false;
  window._curBot = null;
});

document.getElementById('newThread').onclick = () => {
  started = false; transcript = [];
  modeEl.textContent = '将开始新话题';
  bubble('sys', '— 已开新话题 —');
};
document.getElementById('clearImg').onclick = () => { images = []; renderAttach(); };

prompt.addEventListener('keydown', e => {
  if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); doSend(); }
});
send.onclick = doSend;
