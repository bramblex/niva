import {useModel} from "../common/state";
import classNames from "classnames";
import { XPromise } from "../common/utils";

import { ModalComponentProps, ProgressModel } from "../models/modal.model";

import "./style.scss";
import { useLocale, useModal } from "../models/app.model";
import { FormEvent, useEffect, useRef, useState } from "react";

export function NativeModal(_: ModalComponentProps) {
  return <></>;
}

export function AlertModal(
  props: ModalComponentProps & {
    title: string;
    message: string;
    promise: XPromise<void>;
  }
) {
  const { close, title, message, promise } = props;
  const locale = useLocale();
  return (
    <div className="window active is-bright" role="alertdialog" aria-modal="true" aria-label={title}>
      <button
        type="button"
        className="icon-close"
        aria-label={locale.t("CLOSE")}
        onClick={() => {
          close();
          promise.resolve();
        }}
      />
      <div className="window-body has-space">
        <h4>{title}</h4>
        <p>{message}</p>
      </div>
      <footer style={{ textAlign: "right" }}>
        <button
          className="btn btn-md btn-primary"
          autoFocus
          onClick={() => {
            close();
            promise.resolve();
          }}
        >
          {locale.t("CONFIRM")}
        </button>
      </footer>
    </div>
  );
}

export function ConfirmModal(
  props: ModalComponentProps & {
    title: string;
    message: string;
    promise: XPromise<boolean>;
  }
) {
  const { close, title, message, promise } = props;
  const locale = useLocale();
  return (
    <div className="window active is-bright" role="alertdialog" aria-modal="true" aria-label={title} onKeyDown={(event) => {
      if (event.key === "Escape") {
        close();
        promise.resolve(false);
      }
    }}>
      <button
        type="button"
        className="icon-close"
        aria-label={locale.t("CLOSE")}
        onClick={() => {
          close();
          promise.resolve(false);
        }}
      />
      <div className="window-body has-space">
        <h4>{title}</h4>
        <p>{message}</p>
      </div>
      <footer style={{ textAlign: "right" }}>
        <button
          className="btn btn-md"
          autoFocus
          style={{ marginRight: "6px" }}
          onClick={() => {
            close();
            promise.resolve(false);
          }}
        >
          {locale.t("CANCEL")}
        </button>
        <button
          className="btn btn-md btn-primary"
          onClick={() => {
            close();
            promise.resolve(true);
          }}
        >
          {locale.t("CONFIRM")}
        </button>
      </footer>
    </div>
  );
}

export function UnsavedChangesModal({
  close,
  promise,
}: ModalComponentProps & { promise: XPromise<"save" | "discard" | "cancel"> }) {
  const locale = useLocale();
  const decide = (choice: "save" | "discard" | "cancel") => {
    close();
    promise.resolve(choice);
  };

  return (
    <div className="window active is-bright unsaved-dialog" role="alertdialog" aria-modal="true"
      aria-label={locale.t("UNSAVED_TITLE")}
      onKeyDown={(event) => { if (event.key === "Escape") decide("cancel"); }}>
      <div className="window-body has-space">
        <h4>{locale.t("UNSAVED_TITLE")}</h4>
        <p>{locale.t("UNSAVED_MESSAGE")}</p>
      </div>
      <footer className="unsaved-actions">
        <button type="button" className="btn btn-md" autoFocus onClick={() => decide("cancel")}>
          {locale.t("KEEP_EDITING")}
        </button>
        <button type="button" className="btn btn-md btn-danger-text" onClick={() => decide("discard")}>
          {locale.t("DISCARD_CHANGES")}
        </button>
        <button type="button" className="btn btn-md btn-primary" onClick={() => decide("save")}>
          {locale.t("SAVE_CHANGES")}
        </button>
      </footer>
    </div>
  );
}

export function PromptProjectNameModal({
  close,
  promise,
}: ModalComponentProps & { promise: XPromise<string | null> }) {
  const locale = useLocale();
  const [name, setName] = useState("");
  const trimmed = name.trim();
  const isValid = Boolean(trimmed) && trimmed !== "." && trimmed !== ".." &&
    !/[<>:"/\\|?*\x00-\x1F]/.test(trimmed) && !trimmed.endsWith(".");
  const finish = (value: string | null) => {
    close();
    promise.resolve(value);
  };

  return (
    <form className="window active is-bright project-name-dialog" role="dialog" aria-modal="true"
      aria-label={locale.t("NEW_PROJECT")}
      onSubmit={(event) => { event.preventDefault(); if (isValid) finish(trimmed); }}
      onKeyDown={(event) => { if (event.key === "Escape") finish(null); }}>
      <div className="window-body has-space">
        <h4>{locale.t("NEW_PROJECT")}</h4>
        <p>{locale.t("NEW_PROJECT_NAME_HINT")}</p>
        <label className="dialog-field">
          <span>{locale.t("PROJECT_NAME")}</span>
          <input autoFocus value={name} onChange={(event) => setName(event.target.value)}
            placeholder={locale.t("NEW_PROJECT_NAME_PLACEHOLDER")} />
        </label>
        {name && !isValid && <p className="field-error" role="alert">{locale.t("INVALID_PROJECT_NAME")}</p>}
      </div>
      <footer>
        <button type="button" className="btn btn-md" onClick={() => finish(null)}>{locale.t("CANCEL")}</button>
        <button type="submit" className="btn btn-md btn-primary" disabled={!isValid}>{locale.t("CHOOSE_LOCATION")}</button>
      </footer>
    </form>
  );
}

export function ProgressModal({
  close,
  title,
  progress,
}: ModalComponentProps & { title: string; progress: ProgressModel }) {
  useModel(progress);
  const locale = useLocale();
  const { state } = progress;
  const [input, setInput] = useState("");
  const terminalRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (terminalRef.current) {
      terminalRef.current.scrollTop = terminalRef.current.scrollHeight;
    }
  }, [state.commands]);

  const submitInput = (event: FormEvent) => {
    event.preventDefault();
    if (progress.sendInput(input)) setInput("");
  };

  return (
    <div className={classNames("window active is-bright build-progress-modal", {
      "has-terminal": state.commands.length > 0,
    })} role="dialog" aria-modal="true" aria-label={title}>
      <div className="window-body has-space progress">
        <h4 className="instruction instruction-primary">{title}</h4>

        <div className="build-progress-status" aria-live="polite">
          {state.step > 0 && (
            <span>{locale.t("PROGRESS_STEP", { step: String(state.step), total: String(state.total) })}</span>
          )}
          <span className="build-progress-text">{state.text}</span>
        </div>
        <div role="progressbar" className="marquee" aria-label={state.text || title}></div>

        {state.commands.length > 0 && (
          <section className="build-terminal" aria-label={locale.t("BUILD_OUTPUT")}>
            <div className="build-terminal-output" ref={terminalRef}>
              {state.commands.map((command, index) => (
                <article className="build-terminal-command" key={`${index}-${command.label}`}>
                  <header>
                    <strong>{command.label}</strong>
                    <span className={`build-command-status is-${command.status}`}>
                      {command.status === "running"
                        ? locale.t("COMMAND_RUNNING")
                        : command.status === "success"
                          ? locale.t("COMMAND_EXITED", { status: String(command.exitCode ?? 0) })
                          : locale.t("COMMAND_FAILED", { status: String(command.exitCode ?? "?") })}
                    </span>
                  </header>
                  {command.transcript.map((entry, outputIndex) => (
                    <pre className={`is-${entry.kind}`} key={outputIndex}>{entry.text}</pre>
                  ))}
                  {command.transcript.length === 0 && command.status === "running" && (
                    <p className="build-terminal-waiting">{locale.t("COMMAND_WAITING_OUTPUT")}</p>
                  )}
                </article>
              ))}
            </div>

            {state.stdinOpen ? (
              <form className="build-terminal-input" onSubmit={submitInput}>
                <label htmlFor="build-stdin">{locale.t("BUILD_STDIN")}</label>
                <input
                  id="build-stdin"
                  value={input}
                  onChange={(event) => setInput(event.target.value)}
                  placeholder={locale.t("BUILD_STDIN_PLACEHOLDER")}
                  autoComplete="off"
                  autoFocus
                />
                <button className="btn btn-primary" type="submit">{locale.t("SEND_INPUT")}</button>
                <button className="btn" type="button" onClick={() => progress.closeStdin()}>
                  {locale.t("CLOSE_STDIN")}
                </button>
              </form>
            ) : (
              state.commands[state.commands.length - 1]?.status === "running" && (
                <p className="build-terminal-stdin-closed">{locale.t("STDIN_CLOSED")}</p>
              )
            )}
          </section>
        )}
      </div>
    </div>
  );
}

export function Modal() {
  const modal = useModal();
  const modals = modal.state.modals;

  return modals.length > 0 ? (
    <div className="modal-container">
      {modals.map(({ id, Component }, i) => (
        <div
          key={id}
          className={classNames("modal", {
            "modal-active": modals.length === i + 1,
          })}
        >
          <Component />
        </div>
      ))}
    </div>
  ) : null;
}
