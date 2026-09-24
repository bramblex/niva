import "./multi-target-build.scss";

import { FolderOpen } from "@icon-park/react";
import { useState } from "react";
import { pathJoin, tempDirWith, uuid } from "../../common/utils";
import { ErrorCode } from "../../common/error";
import { useApp, useLocale } from "../../models/app.model";
import type { ProjectModel } from "../../models/project.model";
import { extractNodeCompatFiles } from "../../build-scripts/node-compat";

const packagerKitStorageKey = "niva-devtools-packager-kit";
const outputDirectoryStorageKey = "niva-devtools-packager-output";

const targetOptions = [
  { value: "windows-x86_64", label: "Windows x86_64" },
  { value: "macos-aarch64", label: "macOS Apple silicon" },
  { value: "macos-x86_64", label: "macOS Intel" },
] as const;

type BuildTarget = (typeof targetOptions)[number]["value"];
type BuildStatus = "complete" | "failed";

interface TargetResult {
  target: string;
  status: BuildStatus;
  path?: string;
  sha256?: string;
  signature?: string;
  runtimeVersion?: string;
  error?: string;
}

function readSetting(key: string): string {
  try {
    return window.localStorage.getItem(key) ?? "";
  } catch {
    return "";
  }
}

function saveSetting(key: string, value: string) {
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // The current window still keeps the selected path in state.
  }
}

async function hostPackagerName(): Promise<string> {
  const os = (await Niva.api.os.info()).os.toLowerCase().replace(/\s/g, "");
  if (os === "windows") return "niva-packager.exe";
  if (os === "macos") return "niva-packager";
  throw new Error(`Unsupported host operating system: ${os}`);
}

function parseResults(stdout: string, targets: BuildTarget[]): TargetResult[] {
  const lines = stdout.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  const lastLine = lines[lines.length - 1];
  if (!lastLine) throw new Error("The packager returned no result JSON.");

  const response = JSON.parse(lastLine) as { results?: unknown; error?: unknown };
  if (!Array.isArray(response.results)) {
    throw new Error("The packager result must contain a results array.");
  }
  const topLevelError = typeof response.error === "string" ? response.error : undefined;
  if (response.results.length === 0 && topLevelError) {
    return targets.map((target) => ({ target, status: "failed", error: topLevelError }));
  }

  const returned = new Map<string, any>();
  for (const item of response.results) {
    if (item && typeof item.target === "string") returned.set(item.target, item);
  }

  return targets.map((target) => {
    const item = returned.get(target);
    if (!item) {
      return {
        target,
        status: "failed",
        error: topLevelError || "The packager did not return a result for this target.",
      };
    }
    return {
      target,
      status: item.status === "complete" ? "complete" : "failed",
      ...(typeof item.path === "string" ? { path: item.path } : {}),
      ...(typeof item.sha256 === "string" ? { sha256: item.sha256 } : {}),
      ...(typeof item.signature === "string" ? { signature: item.signature } : {}),
      ...(typeof item.runtimeVersion === "string"
        ? { runtimeVersion: item.runtimeVersion }
        : {}),
      ...(typeof item.error === "string" ? { error: item.error } : {}),
    };
  });
}

export function MultiTargetBuildPanel(props: {
  project: ProjectModel;
  onClose: () => void;
}) {
  const { project, onClose } = props;
  const app = useApp();
  const locale = useLocale();
  const [kitDirectory, setKitDirectory] = useState(() =>
    readSetting(packagerKitStorageKey)
  );
  const [outputDirectory, setOutputDirectory] = useState(() =>
    readSetting(outputDirectoryStorageKey)
  );
  const [selectedTargets, setSelectedTargets] = useState<BuildTarget[]>(() =>
    targetOptions.map(({ value }) => value)
  );
  const [results, setResults] = useState<TargetResult[]>([]);
  const [isBuilding, setIsBuilding] = useState(false);
  const [error, setError] = useState("");

  const chooseKitDirectory = async () => {
    setError("");
    const selected = await app.state.modal.showNative(() =>
      Niva.api.dialog.pickDir(kitDirectory || undefined)
    );
    if (!selected) return;

    try {
      const manifestPath = pathJoin(selected, "manifest.json");
      if (!(await Niva.api.fs.exists(manifestPath))) {
        setError(locale.t("PACKAGER_MANIFEST_MISSING"));
        return;
      }
      const manifest = JSON.parse(await Niva.api.fs.read(manifestPath));
      if (!manifest || typeof manifest !== "object" || Array.isArray(manifest)) {
        setError(locale.t("PACKAGER_MANIFEST_INVALID"));
        return;
      }
      const executablePath = pathJoin(selected, await hostPackagerName());
      if (!(await Niva.api.fs.exists(executablePath))) {
        setError(locale.t("PACKAGER_HOST_BINARY_MISSING"));
        return;
      }
      setKitDirectory(selected);
      saveSetting(packagerKitStorageKey, selected);
    } catch (cause) {
      setError(`${locale.t("PACKAGER_KIT_INVALID")} ${String(cause)}`);
    }
  };

  const chooseOutputDirectory = async () => {
    setError("");
    const selected = await app.state.modal.showNative(() =>
      Niva.api.dialog.pickDir(outputDirectory || undefined)
    );
    if (!selected) return;
    setOutputDirectory(selected);
    saveSetting(outputDirectoryStorageKey, selected);
  };

  const openOutputDirectory = async () => {
    setError("");
    try {
      await Niva.api.process.open(outputDirectory);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  };

  const toggleTarget = (target: BuildTarget) => {
    setSelectedTargets((current) =>
      current.includes(target)
        ? current.filter((item) => item !== target)
        : [...current, target]
    );
  };

  const build = async () => {
    setError("");
    setResults([]);

    if (!kitDirectory) {
      setError(locale.t("PACKAGER_SELECT_KIT_FIRST"));
      return;
    }
    if (!outputDirectory) {
      setError(locale.t("PACKAGER_SELECT_OUTPUT_FIRST"));
      return;
    }
    if (selectedTargets.length === 0) {
      setError(locale.t("PACKAGER_SELECT_TARGET"));
      return;
    }
    if (!app.beginPackagerBuild(project)) {
      setError(locale.t("ERR_PACKAGER_BUILD_IN_PROGRESS"));
      return;
    }

    setIsBuilding(true);
    let compatStageDirectory: string | null = null;
    try {
      const projectRefresh = await project.refresh();
      if (projectRefresh.isErr()) {
        if (projectRefresh.error.code !== ErrorCode.PROJECT_HAS_UNSAVED_CHANGE) {
          setError(projectRefresh.error.toLocaleMessage(app));
        }
        return;
      }

      const { process } = Niva.api;
      const executablePath = pathJoin(kitDirectory, await hostPackagerName());
      const manifestPath = pathJoin(kitDirectory, "manifest.json");
      if (!(await Niva.api.fs.exists(manifestPath))) {
        throw new Error(locale.t("PACKAGER_MANIFEST_MISSING"));
      }
      const manifest = JSON.parse(await Niva.api.fs.read(manifestPath));
      if (!manifest || typeof manifest !== "object" || Array.isArray(manifest)) {
        throw new Error(locale.t("PACKAGER_MANIFEST_INVALID"));
      }
      if (!(await Niva.api.fs.exists(executablePath))) {
        throw new Error(locale.t("PACKAGER_HOST_BINARY_MISSING"));
      }
      const args = [
        "build",
        "--manifest",
        manifestPath,
        "--config",
        project.state.configPath,
        "--resource-dir",
        pathJoin(project.state.path, project.state.config.build?.resource),
        "--output-dir",
        outputDirectory,
      ];
      for (const target of selectedTargets) args.push("--target", target);

      if (project.state.config.nodeCompat) {
        compatStageDirectory = tempDirWith(
          "niva-packager-node-compat",
          project.state.uuid,
          uuid()
        );
        const staged = await extractNodeCompatFiles(
          project.state.config.nodeCompat,
          compatStageDirectory
        );
        if (staged) args.push("--node-compat-dir", staged.directory);
      }

      const execution = await process.exec(executablePath, args) as {
        status: number | null;
        stdout: string;
        stderr: string;
      };
      const parsedResults = parseResults(execution.stdout, selectedTargets);
      setResults(parsedResults);
      if (execution.status !== 0 && !parsedResults.some((result) => result.status === "failed")) {
        setError(locale.t("PACKAGER_EXITED_WITH_ERROR"));
      }
      if (execution.status !== 0 && execution.stderr.trim()) {
        setError((current) => current
          ? `${current}\n${execution.stderr.trim()}`
          : execution.stderr.trim());
      }
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      setError(message);
      setResults(selectedTargets.map((target) => ({
        target,
        status: "failed",
        error: message,
      })));
    } finally {
      try {
        if (compatStageDirectory && await Niva.api.fs.exists(compatStageDirectory)) {
          await Niva.api.fs.remove(compatStageDirectory);
        }
      } catch (cleanupError) {
        console.warn("Could not clean staged NodeCompat files", cleanupError);
      }
      setIsBuilding(false);
      app.endPackagerBuild(project);
    }
  };

  return (
    <section className="multi-target-build" aria-labelledby="multi-target-build-title">
      <header className="multi-target-build-header">
        <div>
          <h3 id="multi-target-build-title">{locale.t("PACKAGER_BUILD_TITLE")}</h3>
          <p>{locale.t("PACKAGER_BUILD_HELP")}</p>
        </div>
        <button type="button" className="btn btn-info" disabled={isBuilding} onClick={onClose}>
          {locale.t("CLOSE")}
        </button>
      </header>

      <div className="multi-target-build-settings">
        <div className="packager-setting">
          <div className="packager-setting-label">
            <strong>{locale.t("PACKAGER_KIT")}</strong>
            <span title={kitDirectory || undefined}>
              {kitDirectory || locale.t("PACKAGER_KIT_NOT_SELECTED")}
            </span>
          </div>
          <button type="button" className="btn btn-info" disabled={isBuilding} onClick={() => void chooseKitDirectory()}>
            {locale.t(kitDirectory ? "PACKAGER_CHANGE_KIT" : "PACKAGER_CHOOSE_KIT")}
          </button>
        </div>

        <div className="packager-setting">
          <div className="packager-setting-label">
            <strong>{locale.t("PACKAGER_OUTPUT_DIRECTORY")}</strong>
            <span title={outputDirectory || undefined}>
              {outputDirectory || locale.t("PACKAGER_OUTPUT_NOT_SELECTED")}
            </span>
          </div>
          <button type="button" className="btn btn-info" disabled={isBuilding} onClick={() => void chooseOutputDirectory()}>
            {locale.t(outputDirectory ? "PACKAGER_CHANGE_OUTPUT" : "PACKAGER_CHOOSE_OUTPUT")}
          </button>
        </div>
      </div>

      {!kitDirectory && (
        <p className="packager-inline-hint" role="status">
          {locale.t("PACKAGER_SELECT_KIT_FIRST")}
        </p>
      )}

      <fieldset className="packager-targets" disabled={isBuilding}>
        <legend>{locale.t("PACKAGER_TARGETS")}</legend>
        {targetOptions.map(({ value, label }) => (
          <label key={value} className="packager-target-option">
            <input
              type="checkbox"
              checked={selectedTargets.includes(value)}
              onChange={() => toggleTarget(value)}
            />
            <span>{label}</span>
          </label>
        ))}
      </fieldset>

      {error && <p className="packager-error" role="alert">{error}</p>}

      <footer className="multi-target-build-actions">
        <button
          type="button"
          className="btn btn-info"
          disabled={!outputDirectory || isBuilding}
          onClick={() => void openOutputDirectory()}
        >
          <FolderOpen size={15} />{locale.t("PACKAGER_OPEN_OUTPUT")}
        </button>
        <button
          type="button"
          className="btn btn-primary"
          disabled={isBuilding || !kitDirectory || !outputDirectory || selectedTargets.length === 0}
          onClick={() => void build()}
        >
          {isBuilding ? locale.t("PACKAGER_BUILDING") : locale.t("PACKAGER_START_BUILD")}
        </button>
      </footer>

      {results.length > 0 && (
        <div className="packager-results" aria-live="polite">
          <h4>{locale.t("PACKAGER_RESULTS")}</h4>
          <ul>
            {results.map((result) => (
              <li key={result.target} className={`packager-result ${result.status}`}>
                <div className="packager-result-heading">
                  <strong>{result.target}</strong>
                  <span className={`packager-result-status ${result.status}`}>
                    {locale.t(result.status === "complete" ? "PACKAGER_COMPLETE" : "PACKAGER_FAILED")}
                  </span>
                </div>
                <dl>
                  {result.path && <div><dt>{locale.t("PACKAGER_ARTIFACT")}</dt><dd>{result.path}</dd></div>}
                  {result.sha256 && <div><dt>SHA-256</dt><dd>{result.sha256}</dd></div>}
                  {result.signature && <div><dt>{locale.t("PACKAGER_SIGNATURE")}</dt><dd>{result.signature}</dd></div>}
                  {result.runtimeVersion && <div><dt>{locale.t("PACKAGER_RUNTIME_VERSION")}</dt><dd>{result.runtimeVersion}</dd></div>}
                  {result.error && <div><dt>{locale.t("PACKAGER_ERROR")}</dt><dd>{result.error}</dd></div>}
                </dl>
              </li>
            ))}
          </ul>
        </div>
      )}
    </section>
  );
}
