export type Authority = {
  read_paths: string[];
  write_paths: string[];
  commands: string[];
  network_domains: string[];
  secrets: string[];
  side_effects: string[];
};

export type Task = {
  id: string;
  objective: string;
  dependencies: string[];
  evidence: string[];
  authority: Authority;
  gates: Array<{ id: string; kind: string; approver?: string }>;
  task_digest: string;
  start_line: number;
};

export type CompiledReview = {
  artifact_id: string;
  artifact_digest: string;
  policy_digest: string;
  owner: string;
  status: string;
  risk_class: string;
  objective: string;
  non_goals: string;
  authority: Authority;
  budgets: Record<string, number>;
  evidence: Array<{ id: string; uri: string; digest: string; description: string }>;
  tasks: Task[];
};

export type Diagnostic = { code: string; message: string; start_line?: number };
export type ReviewSnapshot = { valid: boolean; diagnostics: Diagnostic[]; compiled?: CompiledReview };
export type SemanticChange = {
  kind: string;
  path: string;
  detail: string;
  authority_broadening: boolean;
};
