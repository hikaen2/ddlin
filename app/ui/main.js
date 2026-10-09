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
    b.textContent = d.title;
    b.onclick = () => select(d);
    $("dicts").append(b);
  });
  $("hits").innerHTML = "";
  $("count").textContent = "";
  if (!overview.root) message(text, "Choose a dictionary folder to start (Folder…).");
  else if (!dicts.length) message(text, "No dictionaries found in " + overview.root);
  else message(text, "");
  select(dicts.find((d) => d.prefix) || dicts[0] || null);
}

function select(d) {
  dict = d;
  [...$("dicts").children].forEach((b, i) => b.classList.toggle("on", dicts[i] === d));
  for (const b of $("modes").children) b.disabled = !d || !d[b.dataset.mode.toLowerCase()];
  if (d && !d[mode.toLowerCase()]) {
    const m = ["Prefix", "Suffix", "Exact"].find((m) => d[m.toLowerCase()]);
    if (m) mode = m;
  }
  setMode(mode);
  if (d && !(d.prefix || d.suffix || d.exact)) {
    $("hits").innerHTML = "";
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

async function search() {
  const query = $("query").value.trim();
  if (!dict || !query || !dict[mode.toLowerCase()]) return;
  const hits = await invoke("search", { id: dict.id, query, mode });
  const ul = $("hits");
  ul.innerHTML = hits.map((h, i) => `<li data-i="${i}">${h.heading}</li>`).join("");
  ul.scrollTop = 0;
  ul.onclick = (e) => {
    const li = e.target.closest("li");
    if (!li) return;
    ul.querySelector("li.on")?.classList.remove("on");
    li.classList.add("on");
    openAt(hits[li.dataset.i].pos);
  };
  $("count").textContent = hits.length ? `${hits.length} results` : "";
  if (!hits.length) {
    message(text, "No results");
    return;
  }
  ul.firstChild.classList.add("on");
  openAt(hits[0].pos);
}

async function openAt(pos) {
  loading = true;
  try {
    const seg = await invoke("read_segment", { id: dict.id, pos, dir: "Forward" });
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
    const seg = await invoke("read_segment", { id: dict.id, pos: bottomPos, dir: "Forward" });
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
    const seg = await invoke("read_segment", { id: dict.id, pos: topPos, dir: "Backward" });
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
$("folder").onclick = async () => render(await invoke("choose_folder"));

invoke("get_overview").then(render);
