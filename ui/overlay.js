// overlay.js — 区域框选：拖拽选区 + 放大镜 + Esc/Enter
const { invoke } = window.__TAURI__.core;

const cap = document.getElementById('cap');
const rect = document.getElementById('rect');
const mag = document.getElementById('mag');
const magc = document.getElementById('magc');
const sizeEl = document.getElementById('size');
const dim = document.getElementById('dim');

let sx = 0, sy = 0, ex = 0, ey = 0, dragging = false, ready = false;
let imgW = 0, imgH = 0;   // 物理像素

// 页面隐藏时重置；__onShow 由 Rust 调起
window.__onShow = async function () {
  try {
    const cap_data = await invoke('get_capture');
    cap.src = 'data:image/png;base64,' + cap_data.png;
    imgW = cap_data.width; imgH = cap_data.height;
    cap.onload = () => { ready = true; };
    dim.style.background = 'rgba(10,14,23,.35)';
  } catch (e) {
    console.error(String(e));
    hide();
  }
};

function hide() {
  window.__TAURI__.window.getCurrentWindow().hide();
  reset();
}
function reset() {
  rect.style.display = 'none'; mag.style.display = 'none'; sizeEl.style.display = 'none';
  dragging = false;
}

// 物理像素比（CSS px * dpr = 物理 px）
const dpr = () => window.devicePixelRatio || 1;

document.addEventListener('mousedown', e => {
  if (!ready) return;
  dragging = true;
  sx = ex = e.clientX; sy = ey = e.clientY;
  rect.style.display = 'block';
});
document.addEventListener('mousemove', e => {
  if (!ready) return;
  ex = e.clientX; ey = e.clientY;
  drawMag(e.clientX, e.clientY);
  if (dragging) drawRect();
});
document.addEventListener('mouseup', () => { dragging = false; });

function drawRect() {
  const x = Math.min(sx, ex), y = Math.min(sy, ey);
  const w = Math.abs(ex - sx), h = Math.abs(ey - sy);
  Object.assign(rect.style, { left: x + 'px', top: y + 'px', width: w + 'px', height: h + 'px' });
  sizeEl.style.display = 'block';
  sizeEl.style.left = x + 'px';
  sizeEl.style.top = Math.max(0, y - 26) + 'px';
  sizeEl.textContent = Math.round(w * dpr()) + '×' + Math.round(h * dpr());
}

function drawMag(cx, cy) {
  const d = dpr();
  mag.style.display = 'block';
  // 放大镜位于光标右下，贴边翻转
  const mw = 150, mh = 150;
  let mx = cx + 18, my = cy + 18;
  if (mx + mw > innerWidth) mx = cx - mw - 18;
  if (my + mh > innerHeight) my = cy - mh - 18;
  Object.assign(mag.style, { left: mx + 'px', top: my + 'px' });

  const ctx = magc.getContext('2d');
  ctx.imageSmoothingEnabled = false;
  // 取光标周围 15x15 物理像素放大到 150
  const srcSize = 15;
  const px = cx * d - srcSize / 2, py = cy * d - srcSize / 2;
  ctx.clearRect(0, 0, mw, mh);
  ctx.drawImage(cap, px, py, srcSize, srcSize, 0, 0, mw, mh);
  // 中心十字
  ctx.strokeStyle = '#ff6b6b'; ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(mw / 2, mh / 2 - 9); ctx.lineTo(mw / 2, mh / 2 + 9);
  ctx.moveTo(mw / 2 - 9, mh / 2); ctx.moveTo(mw / 2 + 9, mh / 2);
  ctx.stroke();
}

async function confirmSel() {
  const d = dpr();
  let x = Math.min(sx, ex) * d, y = Math.min(sy, ey) * d;
  let w = Math.abs(ex - sx) * d, h = Math.abs(ey - sy) * d;
  x = Math.round(x); y = Math.round(y); w = Math.round(w); h = Math.round(h);
  if (w < 3 || h < 3) { hide(); return; }   // 误点：当取消

  const c = document.createElement('canvas');
  c.width = w; c.height = h;
  c.getContext('2d').drawImage(cap, x, y, w, h, 0, 0, w, h);
  const dataUrl = c.toDataURL('image/png');
  const b64 = dataUrl.split(',')[1];

  hide();
  await invoke('handoff_crop', { pngBase64: b64 });
  const { emit } = window.__TAURI__.event;
  await emit('editor:load', { full: false });
  const wined = window.__TAURI__.window.WebviewWindow.getByLabel('editor');
  await wined.show();
  await wined.setFocus();
}

document.addEventListener('keydown', e => {
  if (e.key === 'Escape') hide();
  if (e.key === 'Enter') confirmSel();
});
// 双击确认
document.addEventListener('dblclick', () => { if (!dragging) confirmSel(); });
