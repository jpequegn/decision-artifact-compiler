import { describe, expect, it } from "vitest";
import { analyzeSource, compareArtifacts } from "./engine";
import { sampleArtifact } from "./sample";

describe("review engine", () => {
  it("compiles the sample into a stable task model", async () => {
    const first = await analyzeSource(sampleArtifact);
    const second = await analyzeSource(sampleArtifact);
    expect(first.valid).toBe(true);
    expect(first.compiled?.tasks.map((task) => task.id)).toEqual(["implement", "inspect", "verify"]);
    expect(first.compiled?.artifact_digest).toBe(second.compiled?.artifact_digest);
  });

  it("flags broadened authority", async () => {
    const base = (await analyzeSource(sampleArtifact)).compiled!;
    const changed = sampleArtifact.replace("network_domains: []", "network_domains: [api.example.com]");
    const current = (await analyzeSource(changed)).compiled!;
    expect(compareArtifacts(base, current).some((item) => item.authority_broadening)).toBe(true);
  });

  it("returns diagnostics for malformed input", async () => {
    expect((await analyzeSource("not an artifact")).diagnostics[0].code).toBe("parse_error");
  });
});
