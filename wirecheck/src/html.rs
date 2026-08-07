//! The dashboard: one inline page, no build step. Polls `/api/state` and
//! renders per-agent cards — auth badge, login controls, check results.

pub const PAGE: &str = r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>wirecheck</title>
<style>
  :root { color-scheme: dark; }
  body { background:#1a1b26; color:#c0caf5; font:14px/1.5 ui-monospace,monospace; margin:0; padding:1.5rem; }
  h1 { font-size:1.1rem; letter-spacing:.06em; }
  h1 .sub { color:#565f89; font-weight:normal; font-size:.8rem; margin-left:.6rem; }
  .cards { display:grid; grid-template-columns:repeat(auto-fit,minmax(340px,1fr)); gap:1rem; }
  .card { background:#16161e; border:1px solid #2f334d; border-radius:.5rem; padding:1rem; }
  .card h2 { margin:0 0 .4rem; font-size:1rem; text-transform:capitalize; display:flex; align-items:center; gap:.5rem; }
  .badge { font-size:.7rem; padding:.1rem .45rem; border-radius:.25rem; background:#2f334d; }
  .badge.ok { color:#9ece6a; } .badge.no { color:#f7768e; }
  .ver { color:#565f89; font-size:.75rem; }
  .login { margin:.6rem 0; padding:.5rem; border:1px dashed #2f334d; border-radius:.4rem; }
  .login a { color:#7aa2f7; word-break:break-all; }
  .login .code { font-size:1.2rem; color:#e0af68; letter-spacing:.15em; }
  button { background:#2f334d; color:#c0caf5; border:0; border-radius:.3rem; padding:.35rem .7rem; cursor:pointer; font:inherit; font-size:.8rem; }
  button:hover { background:#414868; }
  input { background:#1a1b26; color:#c0caf5; border:1px solid #2f334d; border-radius:.3rem; padding:.3rem .5rem; font:inherit; font-size:.8rem; }
  ul.checks { list-style:none; margin:.6rem 0 0; padding:0; }
  ul.checks li { padding:.3rem 0; border-top:1px solid #1f2335; }
  .st { display:inline-block; width:1.2rem; }
  .st.pass { color:#9ece6a; } .st.fail { color:#f7768e; } .st.running { color:#7aa2f7; } .st.skipped { color:#565f89; }
  .what { color:#565f89; font-size:.75rem; }
  .detail { color:#a9b1d6; font-size:.78rem; margin-left:1.2rem; word-break:break-word; }
  .ms { color:#565f89; font-size:.7rem; float:right; }
  .spin { color:#7aa2f7; font-size:.75rem; }
</style>
</head>
<body>
<h1>wirecheck<span class="sub">login + live wire-format checks — agents read /api/state</span>
  <button style="float:right" onclick="post('/api/refresh')">refresh auth</button></h1>
<div class="cards" id="cards"></div>
<script>
const AGENTS = ["claude","codex","muse","opencode"];
async function post(url, body){ await fetch(url,{method:"POST",headers:{"content-type":"application/json"},body:body?JSON.stringify(body):null}); tick(); }
function esc(s){ const d=document.createElement("div"); d.innerText=s??""; return d.innerHTML; }
function icon(st){ return {pass:"✔",fail:"✘",running:"◌",skipped:"–"}[st]||"?"; }

function loginBlock(name, p){
  const l = p.login||{phase:"idle"};
  let inner = "";
  if(l.phase==="await_user"){
    inner += `<div>1. Open <a href="${esc(l.url)}" target="_blank" rel="noopener">${esc(l.url)}</a></div>`;
    if(l.code) inner += `<div>2. Confirm code <span class="code">${esc(l.code)}</span></div>`;
    if(l.needs_code_paste) inner += `<div>2. Paste the code shown after login:
      <input id="${name}-code" size="14"> <button onclick="post('/api/login/${name}/code',{code:document.getElementById('${name}-code').value})">submit</button></div>`;
  } else if(l.phase==="starting" || l.phase==="waiting"){
    inner += `<span class="spin">login flow ${l.phase}…</span>`;
  } else if(l.phase==="done"){
    inner += `<span class="st pass">✔</span> ${esc(l.detail)}`;
  } else if(l.phase==="failed"){
    inner += `<span class="st fail">✘</span> ${esc(l.error)}`;
  }
  let controls = "";
  if(name==="muse"){
    controls = `<button onclick="post('/api/login/muse/device')">browser login</button>
      <input id="muse-key" type="password" placeholder="META_API_KEY" size="18">
      <button onclick="post('/api/login/muse/apikey',{key:document.getElementById('muse-key').value})">set key</button>`;
  } else if(name==="claude"){
    controls = `<button onclick="post('/api/login/claude/start',{mode:'claudeai'})">claude.ai login</button>
      <button onclick="post('/api/login/claude/start',{mode:'console'})">console login</button>`;
  } else if(name==="codex"){
    controls = `<span class="what">login: run <b>codex login</b> on the host (browser callback flow)</span>`;
  } else {
    controls = `<span class="what">no credentials needed — the suite spawns its own <b>opencode serve</b></span>`;
  }
  return `<div class="login">${controls}${inner?"<div style='margin-top:.4rem'>"+inner+"</div>":""}</div>`;
}

function card(name, p){
  const checks = (p.checks||[]).map(c=>`
    <li><span class="st ${c.status}">${icon(c.status)}</span><b>${esc(c.name)}</b>
        ${c.ms!=null?`<span class="ms">${c.ms} ms</span>`:""}
        <div class="what" style="margin-left:1.2rem">${esc(c.what)}</div>
        ${c.detail?`<div class="detail">${esc(c.detail)}</div>`:""}</li>`).join("");
  return `<div class="card">
    <h2>${name}
      <span class="badge ${p.logged_in?"ok":"no"}">${p.logged_in?"authenticated":"no credentials"}</span>
      <span class="ver">${esc(p.binary||"binary not found")}</span></h2>
    <div class="what">${esc(p.auth||"")}</div>
    ${loginBlock(name,p)}
    <button onclick="post('/api/checks/${name}')" ${p.checks_running?"disabled":""}>
      ${p.checks_running?"running…":"run checks"}</button>
    <button onclick="post('/api/suite/${name}')" ${p.cargo_running?"disabled":""}>
      ${p.cargo_running?"cargo tests running…":"run cargo integration tests"}</button>
    <ul class="checks">${checks}</ul>
    ${(p.cargo_tests&&p.cargo_tests.length)||p.cargo_status?`
      <div class="what" style="margin-top:.6rem">cargo integration tests${p.cargo_status?` — ${esc(p.cargo_status)}`:""}</div>
      <ul class="checks">${(p.cargo_tests||[]).map(c=>`
        <li><span class="st ${c.status}">${icon(c.status)}</span>${esc(c.name)}
            ${c.detail?`<div class="detail">${esc(c.detail)}</div>`:""}</li>`).join("")}</ul>`:""}
  </div>`;
}

async function tick(){
  try{
    const s = await (await fetch("/api/state")).json();
    document.getElementById("cards").innerHTML =
      AGENTS.map(a=>card(a, s.agents[a]||{})).join("");
  }catch(e){ /* server restarting; keep polling */ }
}
tick(); setInterval(tick, 1500);
</script>
</body>
</html>
"#;
