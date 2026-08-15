import { createIcons, Check, ChevronRight, CircleAlert, FileCode2, GitCompare, KeyRound, Network, Play, RefreshCw, ShieldCheck } from "lucide";
import "./style.css";
import { analyzeSource, diffSources } from "./engine";
import { sampleArtifact } from "./sample";
import type { CompiledReview, SemanticChange, Task } from "./types";

const root = document.querySelector<HTMLDivElement>("#app")!;
let source = sampleArtifact;
let baseline: CompiledReview;
let current: CompiledReview;
let selectedTask = "inspect";
let activeTab: "review" | "diff" = "review";
let approvedDigest = "";
let semanticChanges: SemanticChange[] = [];

async function initialize() {
  const snapshot = await analyzeSource(source);
  if (!snapshot.compiled) throw new Error("sample artifact must compile");
  baseline = snapshot.compiled;
  current = snapshot.compiled;
  semanticChanges = await diffSources(sampleArtifact, source);
  render([]);
}

async function updateSource(next: string) {
  source = next;
  const snapshot = await analyzeSource(source);
  if (snapshot.compiled) {
    current = snapshot.compiled;
    semanticChanges = await diffSources(sampleArtifact, source);
  }
  render(snapshot.diagnostics);
}

function render(diagnostics: Array<{ code: string; message: string; start_line?: number }>) {
  const task = current.tasks.find((item) => item.id === selectedTask) ?? current.tasks[0];
  const changes = semanticChanges;
  const broadenings = changes.filter((item) => item.authority_broadening);
  const isApproved = diagnostics.length === 0 && approvedDigest === current.artifact_digest;
  root.innerHTML = `
    <header class="topbar">
      <div class="brand"><span class="brand-mark">DA</span><div><strong>Decision Artifact</strong><span>Review workspace</span></div></div>
      <div class="digest"><span>Immutable digest</span><code title="${current.artifact_digest}">${shortDigest(current.artifact_digest)}</code></div>
      <button class="approve ${isApproved ? "approved" : ""}" id="approve" ${diagnostics.length ? "disabled" : ""}>
        <i data-lucide="${isApproved ? "check" : diagnostics.length ? "circle-alert" : "shield-check"}"></i>
        ${diagnostics.length ? "Invalid source" : isApproved ? "Approved" : "Approve digest"}
      </button>
    </header>
    <nav class="tabs" aria-label="Artifact views">
      <button class="tab ${activeTab === "review" ? "active" : ""}" data-tab="review"><i data-lucide="file-code-2"></i>Artifact</button>
      <button class="tab ${activeTab === "diff" ? "active" : ""}" data-tab="diff"><i data-lucide="git-compare"></i>Semantic diff <span>${changes.length}</span></button>
      <div class="status ${diagnostics.length ? "invalid" : "valid"}"><span></span>${diagnostics.length ? "Needs attention" : "Valid and policy-safe"}</div>
    </nav>
    ${diagnostics.length ? diagnosticsView(diagnostics) : ""}
    <main class="workspace">
      <section class="source-pane" aria-label="Artifact source">
        <div class="pane-heading"><div><span>Source</span><strong>${current.artifact_id}.md</strong></div><button class="icon-button" id="reset" title="Reset artifact"><i data-lucide="refresh-cw"></i></button></div>
        <textarea id="source" spellcheck="false" aria-label="Artifact Markdown source">${escapeHtml(source)}</textarea>
      </section>
      <section class="review-pane">
        ${activeTab === "review" ? reviewView(task) : diffView(changes, broadenings)}
      </section>
    </main>`;
  createIcons({ icons: { Check, ChevronRight, CircleAlert, FileCode2, GitCompare, KeyRound, Network, Play, RefreshCw, ShieldCheck } });
  bindEvents();
}

function reviewView(task: Task) {
  return `
    <section class="objective-band">
      <div><span>Objective</span><h1>${escapeHtml(current.objective)}</h1><p>${escapeHtml(current.non_goals)}</p></div>
      <dl><div><dt>Owner</dt><dd>${escapeHtml(current.owner)}</dd></div><div><dt>Risk</dt><dd>${escapeHtml(current.risk_class)}</dd></div><div><dt>Status</dt><dd>${escapeHtml(current.status)}</dd></div></dl>
    </section>
    <section class="graph-section">
      <div class="section-title"><div><span>Compiled plan</span><h2>Task dependency graph</h2></div><strong>${current.tasks.length} tasks</strong></div>
      <div class="dag" role="list" aria-label="Compiled tasks">${current.tasks.map((item) => taskNode(item)).join("")}</div>
    </section>
    <section class="detail-grid">
      <div class="task-inspector">
        <div class="section-title"><div><span>Selected task</span><h2>${escapeHtml(task.id)}</h2></div><code>L${task.start_line}</code></div>
        <p>${escapeHtml(task.objective)}</p>
        <div class="capability-groups">${authorityGroups(task.authority)}</div>
      </div>
      <div class="facts">
        <section><div class="section-title"><h2>Budgets</h2><i data-lucide="play"></i></div><dl class="metrics">${Object.entries(current.budgets).map(([key, value]) => `<div><dt>${label(key)}</dt><dd>${Number(value).toLocaleString()}</dd></div>`).join("")}</dl></section>
        <section><div class="section-title"><h2>Evidence</h2><strong>${current.evidence.length}</strong></div>${current.evidence.map((item) => `<div class="evidence"><span>${escapeHtml(item.id)}</span><strong>${escapeHtml(item.description)}</strong><code>${shortDigest(item.digest)}</code></div>`).join("")}</section>
      </div>
    </section>`;
}

function taskNode(task: Task) {
  return `<button class="task-node ${task.id === selectedTask ? "selected" : ""}" data-task="${escapeHtml(task.id)}" role="listitem">
    <span class="node-index">${String(current.tasks.indexOf(task) + 1).padStart(2, "0")}</span>
    <span class="node-copy"><strong>${escapeHtml(task.id)}</strong><small>${escapeHtml(task.objective)}</small></span>
    <span class="node-meta">${task.dependencies.length ? `after ${task.dependencies.join(", ")}` : "ready"}</span>
    <i data-lucide="chevron-right"></i>
  </button>`;
}

function diffView(changes: SemanticChange[], broadenings: SemanticChange[]) {
  return `<section class="diff-header"><div><span>Compared with loaded baseline</span><h1>Semantic artifact diff</h1><p>${changes.length} material changes, ${broadenings.length} authority broadenings</p></div><div class="diff-digests"><code>${shortDigest(baseline.artifact_digest)}</code><i data-lucide="chevron-right"></i><code>${shortDigest(current.artifact_digest)}</code></div></section>
    <section class="diff-list">${changes.length ? changes.map((item) => `<article class="change ${item.authority_broadening ? "broadening" : ""}"><span>${item.kind}</span><div><strong>${escapeHtml(item.path)}</strong><p>${escapeHtml(item.detail)}</p></div>${item.authority_broadening ? '<i data-lucide="circle-alert"></i>' : ""}</article>`).join("") : '<div class="empty-state"><i data-lucide="shield-check"></i><strong>No semantic changes</strong><span>The current source matches the loaded baseline.</span></div>'}</section>`;
}

function authorityGroups(authority: Task["authority"]) {
  return Object.entries(authority).filter(([, values]) => values.length).map(([key, values]) => `<div><span>${label(key)}</span>${values.map((value) => `<code>${escapeHtml(value)}</code>`).join("")}</div>`).join("") || '<div><span>Authority</span><code>none</code></div>';
}

function diagnosticsView(items: Array<{ code: string; message: string; start_line?: number }>) {
  return `<section class="diagnostics" role="alert"><i data-lucide="circle-alert"></i><div><strong>${items.length} validation ${items.length === 1 ? "issue" : "issues"}</strong>${items.map((item) => `<p><code>${escapeHtml(item.code)}</code>${escapeHtml(item.message)}${item.start_line ? `, line ${item.start_line}` : ""}</p>`).join("")}</div></section>`;
}

function bindEvents() {
  document.querySelector<HTMLTextAreaElement>("#source")?.addEventListener("input", (event) => {
    void updateSource((event.target as HTMLTextAreaElement).value);
  });
  document.querySelector("#reset")?.addEventListener("click", () => { void updateSource(sampleArtifact); });
  document.querySelector("#approve")?.addEventListener("click", () => { approvedDigest = current.artifact_digest; render([]); });
  document.querySelectorAll<HTMLButtonElement>("[data-tab]").forEach((button) => button.addEventListener("click", () => {
    activeTab = button.dataset.tab as typeof activeTab; render([]);
  }));
  const nodes = Array.from(document.querySelectorAll<HTMLButtonElement>("[data-task]"));
  nodes.forEach((button, index) => {
    button.addEventListener("click", () => { selectedTask = button.dataset.task!; render([]); });
    button.addEventListener("keydown", (event) => {
      const next = event.key === "ArrowDown" ? index + 1 : event.key === "ArrowUp" ? index - 1 : index;
      if (next !== index && nodes[next]) { event.preventDefault(); nodes[next].focus(); }
    });
  });
}

function shortDigest(value: string) { return value.replace("sha256:", "").slice(0, 12); }
function label(value: string) { return value.replaceAll("_", " "); }
function escapeHtml(value: string) { return value.replace(/[&<>"']/g, (character) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#039;" })[character]!); }

void initialize();
