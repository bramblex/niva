import { execFile } from "node:child_process";
import { fs, fileExists, path, process } from "../common/node";

export const packagerKitStorageKey = "niva-devtools-packager-kit";

export type BuildTarget = "windows-x86_64" | "macos-aarch64" | "macos-x86_64";
export type ResourceLayout = "embedded" | "external";

export interface PackagerTargetResult {
  target: BuildTarget;
  resourceLayout?: ResourceLayout;
  status: "complete" | "failed";
  path?: string;
  sha256?: string;
  signature?: string;
  runtimeVersion?: string;
  error?: string;
}

export interface PackagerReport {
  results: PackagerTargetResult[];
  error?: string;
}

export interface PackagerExecution {
  report: PackagerReport;
  exitCode: number;
  stderr: string;
}

interface PackagerProject {
  state: {
    path: string;
    configPath: string;
    config: any;
  };
}

export function hostPackagerName(): string {
  if (process.platform === "win32") return "niva-packager.exe";
  if (process.platform === "darwin") return "niva-packager";
  throw new Error(`Unsupported packaging host: ${process.platform}`);
}

export function hostBuildTarget(): BuildTarget {
  if (process.platform === "win32") return "windows-x86_64";
  if (process.platform === "darwin") {
    return process.arch === "arm64" ? "macos-aarch64" : "macos-x86_64";
  }
  throw new Error(`Unsupported packaging host: ${process.platform}`);
}

export async function validatePackagerKit(directory: string): Promise<void> {
  const manifestPath = path.join(directory, "manifest.json");
  const executablePath = path.join(directory, hostPackagerName());
  if (!(await fileExists(manifestPath))) throw new Error("The kit has no manifest.json.");
  if (!(await fileExists(executablePath))) throw new Error("The kit has no host niva-packager executable.");
  const manifest = JSON.parse(await fs.readFile(manifestPath, "utf8"));
  if (!manifest || typeof manifest !== "object" || Array.isArray(manifest) ||
      manifest.schemaVersion !== 1 || !manifest.runtimes || typeof manifest.runtimes !== "object") {
    throw new Error("The packaging manifest is invalid.");
  }
}

export async function runPackager(
  project: PackagerProject,
  kitDirectory: string,
  outputDirectory: string,
  targets: BuildTarget[],
  resourceLayout: ResourceLayout = "embedded",
): Promise<PackagerExecution> {
  if (targets.length === 0) throw new Error("Select at least one target.");
  await validatePackagerKit(kitDirectory);

  const manifestPath = path.join(kitDirectory, "manifest.json");
  const executablePath = path.join(kitDirectory, hostPackagerName());
  const resourceDirectory = path.resolve(
    project.state.path,
    project.state.config.build?.resource || ".",
  );
  const args = [
    "build",
    "--manifest", manifestPath,
    "--config", project.state.configPath,
    "--resource-dir", resourceDirectory,
    "--output-dir", path.resolve(outputDirectory),
    "--resource-layout", resourceLayout,
  ];
  for (const target of targets) args.push("--target", target);

  const execution = await new Promise<{ exitCode: number; stdout: string; stderr: string }>((resolve, reject) => {
    execFile(executablePath, args, { maxBuffer: 4 * 1024 * 1024 }, (error: Error | null, stdout: string, stderr: string) => {
      if (!error) {
        resolve({ exitCode: 0, stdout: String(stdout), stderr: String(stderr) });
        return;
      }
      const status = typeof (error as any).status === "number" ? (error as any).status : 1;
      const output = typeof (error as any).stdout === "string" ? (error as any).stdout : String(stdout || "");
      const errorOutput = typeof (error as any).stderr === "string" ? (error as any).stderr : String(stderr || "");
      if (output.trim()) resolve({ exitCode: status, stdout: output, stderr: errorOutput });
      else reject(error);
    });
  });

  const report = parsePackagerReport(execution.stdout);
  return { report, exitCode: execution.exitCode, stderr: execution.stderr };
}

export function parsePackagerReport(stdout: string): PackagerReport {
  const lastLine = stdout.split(/\r?\n/).map((line) => line.trim()).filter(Boolean).at(-1);
  if (!lastLine) throw new Error("The packager returned no result JSON.");
  const report = JSON.parse(lastLine) as PackagerReport;
  if (!report || !Array.isArray(report.results)) {
    throw new Error("The packager result must contain a results array.");
  }
  return report;
}

export function requireTargetSuccess(
  execution: PackagerExecution,
  target: BuildTarget,
): PackagerTargetResult {
  const result = execution.report.results.find((item) => item.target === target);
  if (execution.exitCode !== 0 || !result || result.status !== "complete" || !result.path) {
    const reason = result?.error || execution.report.error || execution.stderr.trim();
    throw new Error(reason || `The ${target} build failed without a packager result.`);
  }
  return result;
}
