// editor.js — 标注编辑器：框/箭头/文字/马赛克 + 撤销 → 剪贴板+落盘 → 问 codex
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const cv = document.getElementById('cv');
const ctx = cv.getContext('2d');
const win = window.__TAURI__.window.getCurrentWindow();

let base = new Image();       // 原图
let history = [];             // 已完成笔画的快照（dataURL）
let tool = 'rect', color = '#dc2626';
let drawing = false, sx = 0, sy = 0, cur = null;

listen('editor:load', async e => {
  const data = await invoke('get_capture');
  base.onload = () => {
    cv.width = base.naturalWidth; cv.height = base.naturalHeight;
    ctx.drawImage(base, 0, 0);
    history = [cv.toDataURL()];
    // 窗口适配图片尺寸（留边放工具栏）
    const w = Math.min(base.naturalWidth + 80, 1600);
    const h = Math.min(base.naturalHeight + 120, 1000);
    win.setSize(new window.__TAURI__.dpi.LogicalSize(w, h));
    win.center();
    win.show(); win.setFocus();
  };
  base.src = 'data:image/png;base64,' + data.png;
});

// 工具切换
document.querySelectorAll('.t[data-tool]').forEach(b => {
  b.onclick = () => {
    document.querySelectorAll('.t[data-tool]').forEach(x => x.classList.remove('on'));
    b.classList.add('on');
    tool = b.dataset.tool;
    document.getElementById('colorRow').classList.toggle('show', tool !== 'mosaic');
  };
});
document.querySelectorAll('.sw').forEach(s => {
  s.onclick = () => {
    document.querySelectorAll('.sw').forEach(x => x.classList.remove('on'));
    s.classList.add('on');
    color = s.dataset.c;
  };
});

function pt(e) {
  const r = cv.getBoundingClientRect();
  return {
    x: (e.clientX - r.left) * (cv.width / r.width),
    y: (e.clientY - r.top) * (cv.height / r.height),
  };
}

cv.addEventListener('mousedown', e => {
  const p = pt(e);
  if (tool === 'text') {
    const t = prompt('输入文字：');
    if (t) {
      const fs = Math.max(18, cv.width / 40);
      ctx.font = `600 ${fs}px system-ui, 'PingFang SC', sans-serif`;
      ctx.fillStyle = color;
      ctx.textBaseline = 'top';
      ctx.shadowColor = 'rgba(255,255,255,.9)'; ctx.shadowBlur = 4;
      ctx.fillText(t, p.x, p.y);
      ctx.shadowBlur = 0;
      snapshot();
    }
    return;
  }
  drawing = true; sx = p.x; sy = p.y;
  if (tool === 'mosaic') cur = { x: p.x, y: p.y };
});

cv.addEventListener('mousemove', e => {
  if (!drawing) return;
  const p = pt(e);
  restore();
  if (tool === 'rect') {
    ctx.strokeStyle = color; ctx.lineWidth = Math.max(3, cv.width / 400);
    ctx.strokeRect(Math.min(sx, p.x), Math.min(sy, p.y), Math.abs(p.x - sx), Math.abs(p.y - sy));
  } else if (tool === 'arrow') {
    arrow(sx, sy, p.x, p.y);
  } else if (tool === 'mosaic') {
    const x = Math.min(cur.x, p.x), y = Math.min(cur.y, p.y);
    const w = Math.abs(p.x - cur.x), h = Math.abs(p.y - cur.y);
    if (w > 2 && h > 2) pixelate(x, y, w, h);
  }
});

cv.addEventListener('mouseup', () => {
  if (!drawing) return;
  drawing = false;
  snapshot();
});

function arrow(x1, y1, x2, y2) {
  ctx.strokeStyle = color; ctx.fillStyle = color;
  ctx.lineWidth = Math.max(3, cv.width / 400);
  ctx.beginPath(); ctx.moveTo(x1, y1); ctx.lineTo(x2, y2); ctx.stroke();
  const ang = Math.atan2(y2 - y1, x2 - x1);
  const head = Math.max(12, cv.width / 60);
  ctx.beginPath();
  ctx.moveTo(x2, y2);
  ctx.lineTo(x2 - head * Math.cos(ang - Math.PI / 7), y2 - head * Math.sin(ang - Math.PI / 7));
  ctx.lineTo(x2 - head * Math.cos(ang + Math.PI / 7), y2 - head * Math.sin(ang + Math.PI / 7));
  ctx.closePath(); ctx.fill();
}

function pixelate(x, y, w, h) {
  const cell = Math.max(8, Math.round(Math.min(w, h) / 8));
  const tmp = document.createElement('canvas');
  const tw = Math.max(1, Math.round(w / cell)), th = Math.max(1, Math.round(h / cell));
  tmp.width = tw; tmp.height = th;
  const tctx = tmp.getContext('2d');
  tctx.drawImage(cv, x, y, w, h, 0, 0, tw, th);
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(tmp, 0, 0, tw, th, x, y, w, h);
  ctx.imageSmoothingEnabled = true;
}

function snapshot() { history.push(cv.toDataURL()); }
function restore() {
  const last = history[history.length - 1];
  const img = new Image();
  img.onload = () => ctx.drawImage(img, 0, 0);
  img.src = last;
}

document.getElementById('undo').onclick = () => {
  if (history.length > 1) {
    history.pop();
    restore();
  }
};
document.getElementById('cancel').onclick = async () => {
  await win.hide();
};

function toast(msg) {
  const t = document.getElementById('toast');
  t.textContent = msg;
  t.classList.add('show');
  setTimeout(() => t.classList.remove('show'), 2200);
}

document.getElementById('done').onclick = async () => {
  const dataUrl = cv.toDataURL('image/png');
  const b64 = dataUrl.split(',')[1];
  try {
    const r = await invoke('save_result', { pngBase64: b64 });
    toast('已保存并复制 ✓');
    const { emit } = window.__TAURI__.event;
    // 同步给主窗挂到「问问 codex」
    const main = window.__TAURI__.window.WebviewWindow.getByLabel('main');
    await emit('chat:attach', { path: r.path, dataUrl });
    await main.show();
    await main.setFocus();
    setTimeout(async () => { await win.hide(); }, 400);
  } catch (e) {
    toast('保存失败：' + String(e));
  }
};
