import "./multi-target-build.scss";

import { FolderOpen } from "@icon-park/react";
import { useState } from "react";
import { ErrorCode } from "../../common/error";
import { useApp, useLocale } from "../../models/app.model";
import type { ProjectModel } from "../../models/project.model";
import { niva, openExternal } from "../../common/niva";
import {
  runPackager,
  validatePackagerKit,
  packagerKitStorageKey,
  type BuildTarget,
  type PackagerTargetResult,
  type ResourceLayout,
} from "../../build-scripts/packager";

const outputDirectoryStorageKey = "niva-devtools-packager-output";

const targetOptions = [
  { value: "windows-x86_64", label: "Windows x86_64" },
  { value: "macos-aarch64", label: "macOS Apple silicon" },
  { value: "macos-x86_64", label: "macOS Intel" },
] as const;

type BuildStatus = "complete" | "failed";
type TargetResult = PackagerTargetResult & { status: BuildStatus };

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

function parseResults(report: { results: PackagerTargetResult[]; error?: string }, targets: BuildTarget[]): TargetResult[] {
  const topLevelError = report.error;
  const returned = new Map<string, any>();
  for (const item of report.results) {
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
  const [resourceLayout, setResourceLayout] = useState<ResourceLayout>("embedded");
  const [selectedTargets, setSelectedTargets] = useState<BuildTarget[]>(() =>
    targetOptions.map(({ value }) => value)
  );
  const [results, setResults] = useState<TargetResult[]>([]);
  const [isBuilding, setIsBuilding] = useState(false);
  const [error, setError] = useState("");

  const chooseKitDirectory = async () => {
    setError("");
    const selected = await app.state.modal.showNative(() =>
      niva.dialog.pickDir(kitDirectory || undefined)
    );
    if (!selected) return;

    try {
      await validatePackagerKit(selected);
      setKitDirectory(selected);
      saveSetting(packagerKitStorageKey, selected);
    } catch (cause) {
      setError(`${locale.t("PACKAGER_KIT_INVALID")} ${String(cause)}`);
    }
  };

  const chooseOutputDirectory = async () => {
    setError("");
    const selected = await app.state.modal.showNative(() =>
      niva.dialog.pickDir(outputDirectory || undefined)
    );
    if (!selected) return;
    setOutputDirectory(selected);
    saveSetting(outputDirectoryStorageKey, selected);
  };

  const openOutputDirectory = async () => {
    setError("");
    try {
      await openExternal(outputDirectory);
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
    try {
      const projectRefresh = await project.refresh();
      if (projectRefresh.isErr()) {
        if (projectRefresh.error.code !== ErrorCode.PROJECT_HAS_UNSAVED_CHANGE) {
          setError(projectRefresh.error.toLocaleMessage(app));
        }
        return;
      }

      const execution = await runPackager(project, kitDirectory, outputDirectory, selectedTargets, resourceLayout);
      const parsedResults = parseResults(execution.report, selectedTargets);
      setResults(parsedResults);
      if (execution.exitCode !== 0 && !parsedResults.some((result) => result.status === "failed")) {
        setError(locale.t("PACKAGER_EXITED_WITH_ERROR"));
      }
      if (execution.exitCode !== 0 && execution.stderr.trim()) {
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

        <label className="packager-setting">
          <span className="packager-setting-label">
            <strong>{locale.t("PACKAGER_RESOURCE_LAYOUT")}</strong>
          </span>
          <select
            value={resourceLayout}
            disabled={isBuilding}
            onChange={(event) => setResourceLayout(event.target.value as ResourceLayout)}
          >
            <option value="embedded">{locale.t("PACKAGER_EMBEDDED")}</option>
            <option value="external">{locale.t("PACKAGER_EXTERNAL")}</option>
          </select>
        </label>
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
