import { parse } from "yaml";
import initWasm, { diff_artifacts, validate_artifact } from "./wasm-pkg/artifact_wasm";
import type { Authority, CompiledReview, ReviewSnapshot, SemanticChange, Task } from "./types";

let wasmReady: Promise<unknown> | undefined;

const emptyAuthority = (): Authority => ({
  read_paths: [], write_paths: [], commands: [], network_domains: [], secrets: [], side_effects: [],
});

export async function analyzeSource(source: string): Promise<ReviewSnapshot> {
  if (typeof window !== "undefined") {
    try {
      wasmReady ??= initWasm();
      await wasmReady;
      return fromWasmSnapshot(validate_artifact(source));
    } catch (error) {
      return invalid("wasm_error", error instanceof Error ? error.message : String(error));
    }
  }
  return analyzeSourceFallback(source);
}

export async function diffSources(baseSource: string, currentSource: string): Promise<SemanticChange[]> {
  if (typeof window !== "undefined") {
    wasmReady ??= initWasm();
    await wasmReady;
    return (diff_artifacts(baseSource, currentSource) as { changes: SemanticChange[] }).changes;
  }
  const base = await analyzeSourceFallback(baseSource);
  const current = await analyzeSourceFallback(currentSource);
  return base.compiled && current.compiled ? compareArtifacts(base.compiled, current.compiled) : [];
}

async function analyzeSourceFallback(source: string): Promise<ReviewSnapshot> {
  try {
    const front = source.match(/^---\s*\n([\s\S]*?)\n---\s*\n/);
    if (!front) return invalid("parse_error", "missing YAML front matter", 1);
    const metadata = parse(front[1]) as Record<string, unknown>;
    const objective = section(source, "Objective");
    const nonGoals = section(source, "Non-goals");
    const tasks: Task[] = [];
    const taskPattern = /```task\s+([a-zA-Z0-9_-]+)\s*\n([\s\S]*?)```/g;
    for (const match of source.matchAll(taskPattern)) {
      const raw = parse(match[2]) as Record<string, unknown>;
      const authority = normalizeAuthority(raw.authority);
      tasks.push({
        id: match[1], objective: String(raw.objective ?? ""),
        dependencies: strings(raw.dependencies), evidence: strings(raw.evidence), authority,
        gates: Array.isArray(raw.gates) ? raw.gates as Task["gates"] : [],
        task_digest: await digest(JSON.stringify(raw)),
        start_line: source.slice(0, match.index).split("\n").length,
      });
    }
    if (!objective || !nonGoals || tasks.length === 0) {
      return invalid("required_section_missing", "objective, non-goals, and tasks are required");
    }
    const taskIds = new Set(tasks.map((task) => task.id));
    for (const task of tasks) {
      const missing = task.dependencies.find((dependency) => !taskIds.has(dependency));
      if (missing) return invalid("unknown_dependency", `task ${task.id} depends on ${missing}`, task.start_line);
    }
    const authority = normalizeAuthority(metadata.authority);
    const compiled: CompiledReview = {
      artifact_id: String(metadata.id ?? ""), artifact_digest: await digest(normalize(source)),
      policy_digest: await digest(JSON.stringify({ authority, budgets: metadata.budgets, risk: metadata.risk_class })),
      owner: String(metadata.owner ?? ""), status: String(metadata.status ?? ""),
      risk_class: String(metadata.risk_class ?? ""), objective, non_goals: nonGoals, authority,
      budgets: metadata.budgets as Record<string, number>,
      evidence: Array.isArray(metadata.evidence) ? metadata.evidence as CompiledReview["evidence"] : [],
      tasks: tasks.sort((left, right) => left.id.localeCompare(right.id)),
    };
    return { valid: true, diagnostics: [], compiled };
  } catch (error) {
    return invalid("parse_error", error instanceof Error ? error.message : String(error));
  }
}

function fromWasmSnapshot(raw: any): ReviewSnapshot {
  if (!raw.valid || !raw.compiled) return { valid: false, diagnostics: raw.diagnostics ?? [] };
  const item = raw.compiled;
  return {
    valid: true,
    diagnostics: raw.diagnostics ?? [],
    compiled: {
      artifact_id: item.artifact_id,
      artifact_digest: item.artifact_digest,
      policy_digest: item.policy_digest,
      owner: item.owner,
      status: item.status,
      risk_class: item.risk_class,
      objective: item.objective.value,
      non_goals: item.non_goals.value,
      authority: item.authority,
      budgets: item.budgets,
      evidence: item.evidence.map((evidence: any) => ({
        id: evidence.id,
        uri: evidence.uri,
        digest: evidence.declared_digest,
        description: evidence.description,
      })),
      tasks: item.tasks.map((task: any) => ({
        id: task.id,
        objective: task.objective.value,
        dependencies: task.dependencies,
        evidence: task.evidence,
        authority: task.authority,
        gates: task.gates,
        task_digest: task.task_digest,
        start_line: task.source.start_line,
      })),
    },
  };
}

export function compareArtifacts(base: CompiledReview, current: CompiledReview): SemanticChange[] {
  const changes: SemanticChange[] = [];
  if (base.objective !== current.objective) changes.push(change("modified", "objective", "Objective text changed"));
  compareAuthority("authority", base.authority, current.authority, changes);
  if (JSON.stringify(base.budgets) !== JSON.stringify(current.budgets)) {
    changes.push(change("modified", "budgets", "Execution budgets changed"));
  }
  const previous = new Map(base.tasks.map((task) => [task.id, task]));
  for (const task of current.tasks) {
    const before = previous.get(task.id);
    if (!before) changes.push(change("added", `tasks.${task.id}`, "Task added"));
    else compareAuthority(`tasks.${task.id}.authority`, before.authority, task.authority, changes);
  }
  return changes.sort((left, right) => left.path.localeCompare(right.path));
}

function compareAuthority(path: string, base: Authority, current: Authority, changes: SemanticChange[]) {
  const before = new Set(authorityValues(base));
  const after = new Set(authorityValues(current));
  for (const capability of after) if (!before.has(capability)) {
    changes.push(change("added", path, `Capability added: ${capability}`, true));
  }
  for (const capability of before) if (!after.has(capability)) {
    changes.push(change("removed", path, `Capability removed: ${capability}`));
  }
}

function authorityValues(authority: Authority): string[] {
  return Object.entries(authority).flatMap(([kind, values]) => values.map((value) => `${kind}:${value}`));
}

function normalizeAuthority(value: unknown): Authority {
  const authority = value && typeof value === "object" ? value as Record<string, unknown> : {};
  const result = emptyAuthority();
  for (const key of Object.keys(result) as Array<keyof Authority>) result[key] = strings(authority[key]);
  return result;
}

function strings(value: unknown): string[] { return Array.isArray(value) ? value.map(String) : []; }
function normalize(value: string): string { return value.replace(/\r\n/g, "\n").trim(); }
function section(source: string, title: string): string {
  return source.match(new RegExp(`## ${title}\\s*\\n+([\\s\\S]*?)(?=\\n## |$)`))?.[1].trim() ?? "";
}
function invalid(code: string, message: string, start_line?: number): ReviewSnapshot {
  return { valid: false, diagnostics: [{ code, message, start_line }] };
}
function change(kind: string, path: string, detail: string, authority_broadening = false): SemanticChange {
  return { kind, path, detail, authority_broadening };
}
async function digest(value: string): Promise<string> {
  const bytes = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(value));
  return `sha256:${Array.from(new Uint8Array(bytes), (byte) => byte.toString(16).padStart(2, "0")).join("")}`;
}
