const { invoke } = window.__TAURI__.core;
const $ = (id) => document.getElementById(id);

let dicts = [];
let dict = null;          // selected dictionary
let mode = "Prefix";
let topPos = null;           // position to read backward from (null: start reached)
let bottomPos = null;        // position to read forward from (null: end reached)
let loading = false;
let lastScroll = 0;

const text = $("text");

function message(el, s) {
  el.innerHTML = `<p class="message"></p>`;
  el.firstChild.textContent = s;
}

function render(overview) {
  dicts = overview.dicts;
  $("dicts").innerHTML = "";
  dicts.forEach((d) => {
    const b = document.createElement("button");
    b.textContent = Array.from(d.title).slice(0, 3).join("");
    b.onclick = () => select(d);
    $("dicts").append(b);
  });
  $("hits").innerHTML = "";
  hits = [];
  current = -1;
  $("count").textContent = "";
  if (!overview.root) message(text, "Choose a dictionary folder to start (Folder…).");
  else if (!dicts.length) message(text, "No dictionaries found in " + overview.root);
  else message(text, "");
  select(dicts.find((d) => d.prefix) || dicts[0] || null);
}

function select(d) {
  dict = d;
  [...$("dicts").children].forEach((b, i) => b.classList.toggle("on", dicts[i] === d));
  $("status").textContent = d ? d.title : "";
  for (const b of $("modes").children) b.disabled = !d || !d[b.dataset.mode.toLowerCase()];
  if (d && !d[mode.toLowerCase()]) {
    const m = ["Prefix", "Suffix", "Exact"].find((m) => d[m.toLowerCase()]);
    if (m) mode = m;
  }
  setMode(mode);
  if (d && !(d.prefix || d.suffix || d.exact)) {
    $("hits").innerHTML = "";
    hits = [];
    current = -1;
    $("count").textContent = "";
    message(text, "This dictionary has no word search.");
    return;
  }
  if ($("query").value) search();
}

function setMode(m) {
  mode = m;
  for (const b of $("modes").children) b.classList.toggle("on", b.dataset.mode === m && !b.disabled);
}

let hits = [];
let current = -1;         // index of the selected heading

async function search() {
  const query = $("query").value.trim();
  if (!dict || !query || !dict[mode.toLowerCase()]) return;
  hits = await invoke("search", { id: dict.id, query, mode });
  const ul = $("hits");
  ul.innerHTML = hits.map((h, i) => `<li data-i="${i}">${h.heading}</li>`).join("");
  ul.scrollTop = 0;
  $("count").textContent = hits.length ? `${hits.length} results` : "";
  current = -1;
  if (!hits.length) {
    message(text, "No results");
    return;
  }
  pick(0);
}

function pick(i) {
  if (i < 0 || i >= hits.length || i === current) return;
  const ul = $("hits");
  ul.children[current]?.classList.remove("on");
  ul.children[i].classList.add("on");
  ul.children[i].scrollIntoView({ block: "nearest" });
  current = i;
  openAt(hits[i].pos);
}

$("hits").tabIndex = 0;
$("hits").addEventListener("click", (e) => {
  const li = e.target.closest("li");
  if (li) pick(Number(li.dataset.i));
});
$("hits").addEventListener("keydown", (e) => {
  const step = { ArrowDown: 1, ArrowUp: -1 }[e.key];
  if (!step) return;
  e.preventDefault();
  pick(current + step);
});

let opening = 0;

async function openAt(pos) {
  const token = ++opening;
  loading = true;
  try {
    const seg = await invoke("read_segment", { id: dict.id, pos, dir: "Forward" });
    if (token !== opening) return;
    text.innerHTML = seg.html;
    topPos = pos;
    bottomPos = seg.next;
    text.scrollTop = 0;
    lastScroll = 0;
  } finally {
    loading = false;
  }
  fill();
}

// The bottom edge is visible while the text is shorter than the pane.
function fill() {
  if (text.scrollHeight <= text.clientHeight) loadNext();
}

async function loadNext() {
  if (loading || !bottomPos) return;
  loading = true;
  try {
    const token = opening;
    const seg = await invoke("read_segment", { id: dict.id, pos: bottomPos, dir: "Forward" });
    if (token !== opening) return;
    text.insertAdjacentHTML("beforeend", seg.html);
    bottomPos = seg.next;
  } finally {
    loading = false;
  }
  fill();
}

async function loadPrev() {
  if (loading || !topPos) return;
  loading = true;
  try {
    const token = opening;
    const seg = await invoke("read_segment", { id: dict.id, pos: topPos, dir: "Backward" });
    if (token !== opening) return;
    const height = text.scrollHeight;
    text.insertAdjacentHTML("afterbegin", seg.html);
    // Keep the lines the user was looking at in place.
    text.scrollTop += text.scrollHeight - height;
    lastScroll = text.scrollTop;
    topPos = seg.next;
  } finally {
    loading = false;
  }
}

const atTop = () => text.scrollTop <= 0;
const atBottom = () => text.scrollTop + text.clientHeight >= text.scrollHeight - 1;

text.addEventListener("scroll", () => {
  const up = text.scrollTop < lastScroll;
  lastScroll = text.scrollTop;
  if (up && atTop()) loadPrev();
  else if (!up && atBottom()) loadNext();
});
// At an edge the scroll event does not fire, so watch the wheel and keys too.
text.addEventListener("wheel", (e) => {
  if (e.deltaY < 0 && atTop()) loadPrev();
  else if (e.deltaY > 0 && atBottom()) loadNext();
});

// Scroll by exactly what the wheel or touchpad reports, so the WebView
// adds no inertia after the fingers leave the touchpad.
for (const el of [text, $("hits")]) {
  el.addEventListener("wheel", (e) => {
    if (e.ctrlKey) return;
    e.preventDefault();
    const unit = e.deltaMode === 1 ? parseFloat(getComputedStyle(el).fontSize) * 1.5
      : e.deltaMode === 2 ? el.clientHeight : 1;
    el.scrollTop += e.deltaY * unit;
  }, { passive: false });
}
text.tabIndex = 0;
text.addEventListener("keydown", (e) => {
  if (["ArrowUp", "PageUp", "Home"].includes(e.key) && atTop()) loadPrev();
  else if (["ArrowDown", "PageDown", "End", " "].includes(e.key) && atBottom()) loadNext();
});

text.addEventListener("click", (e) => {
  const a = e.target.closest("a[data-pos]");
  if (!a) return;
  const [page, offset] = a.dataset.pos.split(":").map(Number);
  openAt({ page, offset });
});

$("modes").addEventListener("click", (e) => {
  const b = e.target.closest("button");
  if (!b || b.disabled) return;
  setMode(b.dataset.mode);
  search();
});
$("form").addEventListener("submit", (e) => {
  e.preventDefault();
  search();
});
// Drag the splitter to resize the headword pane; the width is remembered.
const side = $("side");
try {
  const w = localStorage.getItem("sideWidth");
  if (w) side.style.width = w;
} catch {}
$("splitter").addEventListener("pointerdown", (e) => {
  e.preventDefault(); // don't select text while dragging
  const splitter = e.currentTarget;
  splitter.setPointerCapture(e.pointerId);
  const move = (e) => {
    side.style.width = Math.max(120, Math.min(e.clientX, innerWidth - 200)) + "px";
  };
  splitter.addEventListener("pointermove", move);
  splitter.addEventListener("pointerup", () => {
    splitter.removeEventListener("pointermove", move);
    try { localStorage.setItem("sideWidth", side.style.width); } catch {}
  }, { once: true });
});

// Fonts: an empty family means the system default, size 0 the default size.
const DEFAULT_SIZE = 14;
let fonts = null;
let families = null;

function applyFonts(f) {
  fonts = f;
  const root = document.documentElement.style;
  for (const k of ["ui", "list", "text"]) {
    const family = f[k].family.replace(/"/g, "");
    root.setProperty(`--${k}-family`, family ? `"${family}", system-ui, sans-serif` : "system-ui, sans-serif");
    root.setProperty(`--${k}-size`, (f[k].size || DEFAULT_SIZE) + "px");
  }
  // Honored by WebKit (Linux, macOS); WebView2 on Windows ignores it.
  document.documentElement.classList.toggle("no-antialias", !f.antialias);
}

$("font").onclick = async () => {
  families ??= await invoke("font_families");
  for (const k of ["ui", "list", "text"]) {
    const select = $(k + "Family");
    select.innerHTML = `<option value="">(Default)</option>`;
    for (const name of families) select.add(new Option(name, name));
    select.value = fonts[k].family;
    $(k + "Size").value = fonts[k].size || DEFAULT_SIZE;
  }
  $("antialias").checked = fonts.antialias;
  $("fonts").showModal();
};

$("fontsCancel").onclick = () => $("fonts").close("cancel");
$("fonts").addEventListener("close", () => {
  if ($("fonts").returnValue !== "ok") return;
  const font = (k) => ({ family: $(k + "Family").value, size: Number($(k + "Size").value) || DEFAULT_SIZE });
  const f = { ui: font("ui"), list: font("list"), text: font("text"), antialias: $("antialias").checked };
  applyFonts(f);
  invoke("set_fonts", { fonts: f });
});

invoke("get_fonts").then(applyFonts);

$("folder").onclick = async () => render(await invoke("choose_folder"));

invoke("get_overview").then(render);
