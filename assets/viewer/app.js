/* Inspired by Ontology 1.0.0's offline MCP Apps viewer (MIT). No network calls. */
(() => {
  "use strict";
  const $ = id => document.getElementById(id);
  const node = (tag, text) => { const e = document.createElement(tag); if (text != null) e.textContent = text; return e; };
  const number = value => typeof value === "number" && Number.isFinite(value) ? value.toLocaleString(undefined,Number.isInteger(value) ? {} : {maximumSignificantDigits:6}) : "Unknown";
  const send = message => window.parent.postMessage({ jsonrpc: "2.0", ...message }, "*");
  let ready = false, mount, height = 0;
  function context(value) { if (["dark", "light"].includes(value?.theme)) document.documentElement.dataset.theme = value.theme; }
  function failure(message) { $("status").hidden = false; $("status").textContent = message; }
  function charts(value,calibration) {
    const target=$(calibration ? "calibration" : "thresholds");target.replaceChildren();
    const rows=value.datasets.find(d=>d.id===(calibration ? "reliability" : "development-thresholds"))?.rows || [];
    if(rows.length>4096){target.append(node("p","Chart rows exceed the display bound. Read the typed data."));return;}
    const detail=calibration ? "Raw and out-of-fold recalibrated scores. The dashed diagonal is ideal calibration. Small bins are descriptive; inspect Brier losses and per-fold metrics in the saved report." : `Full-development curves, not held-out performance. Requirements: precision ≥ ${number(value.protocol?.min_precision)}, recall ≥ ${number(value.protocol?.min_recall)}. The eval ledger uses training-only thresholds on held-out folds.`;
    const note=node("p",detail);note.className="chart-note";target.append(note);
    const legend=node("div");legend.className="legend";
    for(const label of calibration ? ["Raw score","Recalibrated, out of fold"] : ["Precision","Recall"])legend.append(node("span",label));target.append(legend);
    const grid=node("div");grid.className="plots";target.append(grid);
    const names=[...new Set(rows.map(r=>r.id))].slice(0,16);
    const svgNode=(tag,attrs,text)=>{const e=document.createElementNS("http://www.w3.org/2000/svg",tag);for(const[k,v]of Object.entries(attrs||{}))e.setAttribute(k,String(v));if(text!=null)e.textContent=text;return e;};
    const finite=p=>typeof p==="number"&&Number.isFinite(p)&&p>=0&&p<=1;
    for(const id of names){
      const panel=node("article");panel.className="plot";panel.append(node("h2",id));
      const svg=svgNode("svg",{viewBox:"0 0 420 250",role:"img","aria-label":`${calibration ? "Calibration" : "Threshold"} chart for ${id}`});
      const x=p=>38+350*p,y=p=>205-175*p;
      svg.append(svgNode("path",{d:"M38 30V205H388",class:"axis"}));
      for(const t of[0,0.5,1]){svg.append(svgNode("text",{x:x(t),y:225,"text-anchor":"middle"},t));svg.append(svgNode("text",{x:29,y:y(t)+3,"text-anchor":"end"},t));}
      svg.append(svgNode("text",{x:213,y:245,"text-anchor":"middle"},calibration ? "Mean predicted failure score" : "Failure threshold"));
      if(calibration)svg.append(svgNode("path",{d:`M${x(0)} ${y(0)}L${x(1)} ${y(1)}`,class:"axis","stroke-dasharray":"4 4"}));
      const own=rows.filter(r=>r.id===id);
      for(const [index,key]of(calibration ? ["raw","platt_oof"] : ["precision","recall"]).entries()){
        const color=index ? "var(--bad)" : "var(--accent)";
        const points=(calibration ? own.filter(r=>r.method===key).map(r=>({x:r.mean_score,y:r.observed_failure_rate,n:r.n})) : own.map(r=>({x:r.threshold,y:r[key]}))).sort((a,b)=>a.x-b.x);
        let segment=[];const flush=()=>{if(segment.length)svg.append(svgNode("polyline",{points:segment.join(" "),class:"curve",stroke:color}));segment=[];};
        for(const p of points){if(!finite(p.x)||!finite(p.y)){flush();continue;}segment.push(`${x(p.x)},${y(p.y)}`);if(calibration){const circle=svgNode("circle",{cx:x(p.x),cy:y(p.y),r:4,fill:color,class:"point"});circle.append(svgNode("title",{},`n=${p.n}; score=${number(p.x)}; observed=${number(p.y)}`));svg.append(circle);}}
        flush();
      }
      panel.append(svg);grid.append(panel);
    }
    if(!names.length)target.append(node("p","No supported calibration or threshold data in this view."));
  }
  function show(value) {
    mount?.destroy(); mount = null; $("diagram").replaceChildren();
    if (value?.schema !== "paperhands.view/v1" || !Array.isArray(value.datasets?.[0]?.rows)) {
      $("desk").hidden = true; failure("This result has no supported Paperhands evidence view."); return;
    }
    const rows = value.datasets[0].rows;
    const objectView=value.kind === "graph";
    const judgeView=value.kind === "judge";
    const tabs=[...document.querySelectorAll("[data-tab]")];tabs[1].hidden=objectView||judgeView;tabs[2].textContent=objectView ? "Object ledger" : judgeView ? "Eval ledger" : "Process ledger";tabs[3].hidden=tabs[4].hidden=!judgeView;tabs[0].click();
    if (rows.length > 50 || typeof value.presentation?.source !== "string" || value.presentation.source.length > 20000) {
      $("desk").hidden = true; failure("Evidence view exceeds its rendering bounds."); return;
    }
    $("status").hidden = true; $("desk").hidden = false;
    $("title").textContent = value.kind === "desk" ? "Research desk" : objectView ? "Research map" : judgeView ? "Decision evals" : "Evidence desk";
    $("warning").textContent = objectView ? "p = declared attention. s = measured semantic similarity. Follow exact revision citations; neither weight establishes truth or independent evidence." : judgeView ? "Model scores, tuned thresholds and recalibrated probabilities are separate evidence. Eligibility does not authorize action. Null metrics remain unsupported or unknown." : "Process measurements describe execution. They do not establish financial fills, profit, or independent policies. Null measurements remain unknown.";
    $("subtitle").textContent = `${value.kind} · ${value.id} · ${value.integrity_status}`;
    $("research").hidden = value.kind !== "desk";
    if (value.kind === "desk") {
      const desk = value.desk;
      $("stats").replaceChildren(); $("frontier").replaceChildren(); $("questions").replaceChildren();
      for (const [label, amount] of [["Retained notes",desk.notes_total],["Open questions",desk.note_kinds == null ? null : desk.note_kinds.question || 0],["Research tasks",desk.tasks_total],["Recorded attempts / batches",desk.coverage.total_attempts_and_batches]]) {
        const stat = node("div"); stat.className = "stat"; stat.append(node("strong",number(amount)),node("span",label)); $("stats").append(stat);
      }
      function card(target, title, detail, argv) {
        const c = node("article"); c.className = "card"; c.append(node("strong",title),node("p",detail));
        if (Array.isArray(argv)) {
          const copy = node("button","Copy MCP args"); copy.className="copy"; copy.type="button";
          copy.onclick=async()=>{const text=JSON.stringify({argv:argv.slice(1)});try{await navigator.clipboard.writeText(text);copy.textContent="Copied";}catch{c.append(node("pre",text));copy.disabled=true;}};
          c.append(copy);
        }
        $(target).append(c);
      }
      for (const item of desk.frontier.slice(0,3)) card("frontier",item.title || item.id,item.attention?.priority == null ? item.reason : `${item.reason} · Declared attention ${item.attention.priority.toFixed(2)}`,item.argv);
      if (desk.frontier.length>3) {const more=node("p",`${desk.frontier.length-3} more items in this inspected frontier. Read the structured desk to continue.`);more.className="empty";$("frontier").append(more);}
      if (!desk.frontier.length) { const empty=node("p","No ready work recorded in this inspected scope. Add a question, inspect other pages, or record the next task.");empty.className="empty";$("frontier").append(empty); }
      for (const question of desk.questions.slice(0,6)) card("questions",question.title,`Retained source · ${question.id.slice(0,12)}`, ["paperhands","memory","read","--root",desk.root,question.id]);
      if (!desk.questions.length) {const empty=node("p","Record the question before choosing an answer.");empty.className="empty";$("questions").append(empty);}
      if (desk.gaps.length) failure(`${desk.gaps.length} retained integrity gap(s). Affected state is unknown; inspect the structured gaps before using this evidence.`);
    }
    $("coverage").textContent = `${rows.length} of ${value.coverage.total} rows · offset ${value.coverage.offset} · ${value.review_scope}. Generated ${new Date(value.generated_at_ms).toISOString()}.`;
    $("source").textContent = value.presentation.source;
    $("duration").replaceChildren(); $("table").replaceChildren();
    const table = node("table"), header = node("tr");
    const columns=objectView||judgeView ? value.datasets[0].columns.map(c=>[c.name,c.type === "number"]) : [["id",false],["process_status",false],["evidence_status",false],["exit_code",true],["wall_time_ms",true],["cpu_time_ms",true],["peak_rss_bytes",true]];
    for (const [label] of columns) header.append(node("th",label.replaceAll("_"," ")));
    const thead = node("thead"); thead.append(header); table.append(thead);
    const body = node("tbody"), max = Math.max(1, ...rows.map(r => typeof r.wall_time_ms === "number" && Number.isFinite(r.wall_time_ms) ? r.wall_time_ms : 0));
    for (const row of rows) {
      const tr = node("tr");
      for (const [key, numeric] of columns) {
        tr.append(node("td", numeric ? number(row[key]) : row[key] ?? "Unknown"));
      }
      body.append(tr);
      const barRow = node("div"), track = node("div"), bar = node("div"), amount = node("span", number(row.wall_time_ms));
      barRow.className = "bar-row"; barRow.dataset.status = row.process_status;
      track.className = "track"; bar.className = "bar"; amount.className = "number";
      bar.style.width = `${typeof row.wall_time_ms === "number" && Number.isFinite(row.wall_time_ms) ? Math.max(0,Math.min(100,row.wall_time_ms/max*100)) : 0}%`;
      track.append(bar); barRow.append(node("span",row.id),track,amount); $("duration").append(barRow);
    }
    table.append(body); $("table").append(table);
    if (!rows.length) $("duration").append(node("p","No measured commands on this page. Unfinished work and unknown measurements remain explicit."));
    if(judgeView){charts(value,false);charts(value,true);if(value.datasets[1]?.rows?.length)tabs[3].click();}
    try {
      mount = window.pele.mount($("diagram"), value.presentation.source, {
        limit: 20000, maxEdges: 256, outputLimit: 750000, links: false, images: false,
        fontFamily: "system-ui, sans-serif",
        sanitize: svg => window.DOMPurify.sanitize(svg, { RETURN_DOM_FRAGMENT:true, USE_PROFILES:{svg:true,svgFilters:true}, FORBID_TAGS:["script","foreignObject","a","image","style","animate","animateMotion","animateTransform","set"] }),
        onError: error => failure(`Diagram unavailable: ${error.message || error}. Original Mermaid and typed data remain available.`)
      });
    } catch (error) { failure(`Diagram unavailable: ${error.message || error}.`); }
  }
  for (const button of document.querySelectorAll("[data-tab]")) button.onclick = () => {
    for (const candidate of document.querySelectorAll("[data-tab]")) {
      const selected = candidate === button; candidate.setAttribute("aria-pressed", String(selected)); $(candidate.dataset.tab).hidden = !selected;
    }
  };
  window.addEventListener("message", event => {
    if (event.source !== window.parent || event.data?.jsonrpc !== "2.0") return;
    const message = event.data, params = message.params || {};
    if (message.id === "paperhands-ui-init") {
      if (message.error) { failure("The MCP host could not initialize this evidence viewer."); return; }
      ready = true; context(message.result?.hostContext); send({method:"ui/notifications/initialized"});
    } else if (message.method === "ping" && message.id != null) send({id:message.id,result:{}});
    else if (message.method === "ui/resource-teardown" && message.id != null) { mount?.destroy(); ready=false; send({id:message.id,result:{}}); }
    else if (ready && message.method === "ui/notifications/host-context-changed") context(params);
    else if (ready && message.method === "ui/notifications/tool-result") {
      if (params.isError) { $("desk").hidden = true; mount?.destroy(); failure("Evidence verification failed. Read the tool error; no intact result is implied."); }
      else show(params.structuredContent?.data);
    } else if (ready && message.method === "ui/notifications/tool-cancelled") failure("View cancelled.");
  });
  new ResizeObserver(() => {
    const next = Math.ceil(document.body.getBoundingClientRect().height);
    if (ready && height !== next) { height=next; send({method:"ui/notifications/size-changed",params:{height:Math.min(next,1000)}}); }
  }).observe(document.body);
  if (window.parent !== window) send({id:"paperhands-ui-init",method:"ui/initialize",params:{protocolVersion:"2026-01-26",appInfo:{name:"paperhands-evidence",version:"0.4.0"},appCapabilities:{availableDisplayModes:["inline"]}}});
  else failure("Open this embedded viewer through an MCP Apps host.");
})();
